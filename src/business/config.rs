use serde::{Deserialize, Serialize};

use crate::foundation::types::constants;

/// 读取器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderConfig {
    /// 缓冲区大小（字节）
    pub buffer_size: usize,
    /// 索引缓存大小（条目数）
    pub index_cache_size: usize,
}

impl Default for ReaderConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,
            index_cache_size: 1000,
        }
    }
}

impl ReaderConfig {
    /// 验证读取器配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.buffer_size < 1024 {
            return Err(
                "缓冲区大小不能小于1024字节".to_string()
            );
        }

        if self.buffer_size > constants::MAX_BUFFER_SIZE {
            return Err(format!(
                "缓冲区大小不能超过{}字节",
                constants::MAX_BUFFER_SIZE
            ));
        }

        if self.index_cache_size == 0 {
            return Err("索引缓存大小必须大于0".to_string());
        }

        Ok(())
    }

    /// 重置为默认值
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// 写入器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriterConfig {
    /// 缓冲区大小（字节）
    pub buffer_size: usize,
    /// 索引缓存大小（条目数）
    pub index_cache_size: usize,
    /// 每个PCAP文件最大数据包数量
    pub max_packets_per_file: usize,
    /// 每个PCAP文件最大大小（字节），0表示不限制
    pub max_file_size_bytes: u64,
    /// 文件命名格式
    pub file_name_format: String,
    /// 是否启用自动刷新
    pub auto_flush: bool,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,
            index_cache_size: 1000,
            max_packets_per_file:
                constants::DEFAULT_MAX_PACKETS_PER_FILE,
            max_file_size_bytes: 0, // 默认不限制文件大小
            file_name_format:
                constants::DEFAULT_FILE_NAME_FORMAT
                    .to_string(),
            auto_flush: true,
        }
    }
}

impl WriterConfig {
    /// 验证写入器配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.buffer_size < 1024 {
            return Err(
                "缓冲区大小不能小于1024字节".to_string()
            );
        }

        if self.buffer_size > constants::MAX_BUFFER_SIZE {
            return Err(format!(
                "缓冲区大小不能超过{}字节",
                constants::MAX_BUFFER_SIZE
            ));
        }

        if self.index_cache_size == 0 {
            return Err("索引缓存大小必须大于0".to_string());
        }

        if self.max_packets_per_file == 0 {
            return Err("每个文件最大数据包数量必须大于0"
                .to_string());
        }

        if self.max_file_size_bytes > 0
            && self.max_file_size_bytes < 1024
        {
            return Err(
                "文件大小限制不能小于1024字节".to_string()
            );
        }

        if self.file_name_format.is_empty() {
            return Err("文件命名格式不能为空".to_string());
        }

        Ok(())
    }

    /// 重置为默认值
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
