use indicatif::{ProgressBar, ProgressStyle};
use tokio::task::JoinHandle;

/// 进度指示器（旋转动画）
pub struct Spinner {
    pb: ProgressBar,
    base_message: String,
    time_task: Option<JoinHandle<()>>,
    #[allow(dead_code)]
    colored: bool,
}

impl Spinner {
    /// 创建新的 spinner
    pub fn new(message: &str, colored: bool) -> Self {
        let pb = ProgressBar::new_spinner();
        let template = if colored {
            "{spinner:.green} {msg:.cyan}"
        } else {
            "{spinner} {msg}"
        };
        pb.set_style(
            ProgressStyle::default_spinner()
                .template(template)
                .expect("Invalid template"),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Self {
            pb,
            base_message: message.to_string(),
            time_task: None,
            colored,
        }
    }

    /// 创建带取消提示的 spinner
    pub fn new_with_cancel_hint(message: &str, colored: bool) -> Self {
        use rust_i18n::t;

        let pb = ProgressBar::new_spinner();
        let template = if colored {
            "{spinner:.green} {msg:.cyan}"
        } else {
            "{spinner} {msg}"
        };
        pb.set_style(
            ProgressStyle::default_spinner()
                .template(template)
                .expect("Invalid template"),
        );
        let display_message = format!("{} {}", message, t!("spinner.cancel_hint"));
        pb.set_message(display_message);
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Self {
            pb,
            base_message: message.to_string(),
            time_task: None,
            colored,
        }
    }

    /// 启动时间显示（每秒更新一次）
    pub fn start_time_display(&mut self) {
        use rust_i18n::t;

        let pb = self.pb.clone();
        let base_msg = self.base_message.clone();

        let handle = tokio::spawn(async move {
            let start = std::time::Instant::now();
            loop {
                let elapsed = start.elapsed().as_secs();
                pb.set_message(format!(
                    "{} {} {}",
                    base_msg,
                    t!("spinner.cancel_hint"),
                    t!("spinner.waiting", seconds = elapsed)
                ));
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        self.time_task = Some(handle);
    }

    /// 停止时间显示
    fn stop_time_display(&mut self) {
        if let Some(handle) = self.time_task.take() {
            handle.abort();
        }
    }

    /// 更新 spinner 消息
    #[allow(dead_code)]
    pub fn set_message(&self, message: &str) {
        self.pb.set_message(message.to_string());
    }

    /// 在基础消息后追加后缀
    pub fn append_suffix(&self, suffix: &str) {
        let full_message = format!("{} {}", self.base_message, suffix);
        self.pb.set_message(full_message);
    }

    /// 完成并显示最终消息
    #[allow(dead_code)]
    pub fn finish_with_message(&self, message: &str) {
        self.pb.finish_with_message(message.to_string());
    }

    /// 完成并清除
    pub fn finish_and_clear(&self) {
        self.pb.finish_and_clear();
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop_time_display();
        self.pb.finish_and_clear();
    }
}
