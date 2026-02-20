use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// 截断消息以适应终端宽度
fn truncate_to_width(msg: &str, available: usize) -> String {
    if available == 0 || console::measure_text_width(msg) <= available {
        msg.to_string()
    } else {
        console::truncate_str(msg, available, "…").to_string()
    }
}

/// 计算 `content_width` 宽的内容在 `term_width` 列终端下占几个物理行
fn physical_lines(content_width: usize, term_width: usize) -> usize {
    if term_width == 0 || content_width == 0 {
        return 1;
    }
    // ceiling division
    content_width.div_ceil(term_width)
}

/// 向上移动光标 n 行并清除到屏幕底部
fn move_up_and_clear(stderr: &mut io::Stderr, lines_up: usize) {
    if lines_up > 0 {
        let _ = write!(stderr, "\x1b[{}A", lines_up);
    }
    let _ = write!(stderr, "\r\x1b[J");
}

/// Progress indicator (rotation animation)
///
/// 自行管理渲染循环。每次 tick：
/// 1. 用上一次渲染宽度 + 当前终端宽度算出 reflow 后的物理行数
/// 2. 向上移动到起始行
/// 3. `\r\x1b[J` 清除到屏幕底部
/// 4. 写入截断后的新内容
pub struct Spinner {
    message: Arc<Mutex<String>>,
    base_message: String,
    running: Arc<AtomicBool>,
    /// 上一次渲染的显示宽度，用于 finish/drop 时清除残留
    prev_width: Arc<AtomicUsize>,
    spin_task: Option<JoinHandle<()>>,
    time_task: Option<JoinHandle<()>>,
    #[allow(dead_code)]
    colored: bool,
}

impl Spinner {
    /// Create new spinner
    pub fn new(message: &str, colored: bool) -> Self {
        let msg = Arc::new(Mutex::new(message.to_string()));
        let running = Arc::new(AtomicBool::new(true));
        let prev_width = Arc::new(AtomicUsize::new(0));
        let spin_task =
            Self::spawn_render_loop(msg.clone(), running.clone(), prev_width.clone(), colored);

        Self {
            message: msg,
            base_message: message.to_string(),
            running,
            prev_width,
            spin_task: Some(spin_task),
            time_task: None,
            colored,
        }
    }

    /// Create a spinner with cancellation prompt
    pub fn new_with_cancel_hint(message: &str, colored: bool) -> Self {
        use rust_i18n::t;

        let display_message = format!("{} {}", message, t!("spinner.cancel_hint"));
        let msg = Arc::new(Mutex::new(display_message));
        let running = Arc::new(AtomicBool::new(true));
        let prev_width = Arc::new(AtomicUsize::new(0));
        let spin_task =
            Self::spawn_render_loop(msg.clone(), running.clone(), prev_width.clone(), colored);

        Self {
            message: msg,
            base_message: message.to_string(),
            running,
            prev_width,
            spin_task: Some(spin_task),
            time_task: None,
            colored,
        }
    }

    fn spawn_render_loop(
        message: Arc<Mutex<String>>,
        running: Arc<AtomicBool>,
        prev_width: Arc<AtomicUsize>,
        colored: bool,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut idx = 0usize;
            let mut stderr = io::stderr();

            while running.load(Ordering::SeqCst) {
                let ch = SPINNER_CHARS[idx % SPINNER_CHARS.len()];
                let msg_str = message.lock().unwrap().clone();

                let term_width = console::Term::stderr().size().1 as usize;

                // 上一次渲染在 **当前** 终端宽度下 reflow 后占几行
                let old_w = prev_width.load(Ordering::SeqCst);
                let old_lines = physical_lines(old_w, term_width);
                // 向上移动到上一次渲染的起始行，然后清除
                move_up_and_clear(&mut stderr, old_lines.saturating_sub(1));

                // 截断消息，保证本次渲染不 wrap
                let available = term_width.saturating_sub(2); // spinner char + space
                let display_msg = truncate_to_width(&msg_str, available);
                let content_width = 2 + console::measure_text_width(&display_msg);

                if colored {
                    let _ = write!(
                        stderr,
                        "\x1b[32m{}\x1b[0m \x1b[36m{}\x1b[0m",
                        ch, display_msg
                    );
                } else {
                    let _ = write!(stderr, "{} {}", ch, display_msg);
                }
                let _ = stderr.flush();

                prev_width.store(content_width, Ordering::SeqCst);
                idx = idx.wrapping_add(1);
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
        })
    }

    /// Startup time display (updated every second)
    pub fn start_time_display(&mut self) {
        use rust_i18n::t;

        let message = self.message.clone();
        let base_msg = self.base_message.clone();
        let running = self.running.clone();

        let handle = tokio::spawn(async move {
            let start = std::time::Instant::now();
            while running.load(Ordering::SeqCst) {
                let elapsed = start.elapsed().as_secs();
                let new_msg = format!(
                    "{} {} {}",
                    base_msg,
                    t!("spinner.cancel_hint"),
                    t!("spinner.waiting", seconds = elapsed)
                );
                *message.lock().unwrap() = new_msg;
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        self.time_task = Some(handle);
    }

    /// Stop time display
    fn stop_time_display(&mut self) {
        if let Some(handle) = self.time_task.take() {
            handle.abort();
        }
    }

    fn stop_render(&mut self) {
        if let Some(handle) = self.spin_task.take() {
            handle.abort();
        }
    }

    /// 清除 spinner 残留（考虑 reflow）
    fn clear_output(&self) {
        let mut stderr = io::stderr();
        let term_width = console::Term::stderr().size().1 as usize;
        let old_w = self.prev_width.load(Ordering::SeqCst);
        let old_lines = physical_lines(old_w, term_width);
        move_up_and_clear(&mut stderr, old_lines.saturating_sub(1));
        let _ = stderr.flush();
    }

    /// Update spinner message
    #[allow(dead_code)]
    pub fn set_message(&self, message: &str) {
        *self.message.lock().unwrap() = message.to_string();
    }

    /// Append suffix after basic message
    pub fn append_suffix(&self, suffix: &str) {
        let full_message = format!("{} {}", self.base_message, suffix);
        *self.message.lock().unwrap() = full_message;
    }

    /// Complete and display final message
    #[allow(dead_code)]
    pub fn finish_with_message(&self, message: &str) {
        self.running.store(false, Ordering::SeqCst);
        self.clear_output();
        let mut stderr = io::stderr();
        let _ = writeln!(stderr, "{}", message);
        let _ = stderr.flush();
    }

    /// Complete and clear
    pub fn finish_and_clear(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.clear_output();
    }
}

impl crate::llm::ProgressReporter for Spinner {
    fn append_suffix(&self, suffix: &str) {
        Spinner::append_suffix(self, suffix);
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop_time_display();
        self.stop_render();
        // 只有还在跑的时候才清除，避免覆盖 finish_with_message 的输出
        if self.running.swap(false, Ordering::SeqCst) {
            self.clear_output();
        }
    }
}
