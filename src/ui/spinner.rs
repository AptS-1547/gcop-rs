use indicatif::{ProgressBar, ProgressStyle};

/// 进度指示器（旋转动画）
pub struct Spinner {
    pb: ProgressBar,
    base_message: String,
}

impl Spinner {
    /// 创建新的 spinner
    pub fn new(message: &str) -> Self {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("Invalid template"),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Self {
            pb,
            base_message: message.to_string(),
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
        self.pb.finish_and_clear();
    }
}
