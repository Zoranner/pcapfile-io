use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::foundation::types::constants;

/// 通用配置 - 读写器都需要的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonConfig {
    /// 缓冲区大小（字节）
    pub buffer_size: usize,
    /// 最大数据包大小（字节）
    pub max_packet_size: usize,
    /// 是否启用数据验证
    pub enable_validation: bool,
    /// 是否启用压缩
    pub enable_compression: bool,
    /// 索引缓存大小（条目数）
    pub index_cache_size: usize,
    /// 是否启用文件索引缓存
    pub enable_index_cache: bool,
    /// 临时目录路径
    pub temp_directory: PathBuf,
}

impl Default for CommonConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,
            max_packet_size: constants::MAX_PACKET_SIZE,
            enable_validation: true,
            enable_compression: false,
            index_cache_size: 1000,
            enable_index_cache: true,
            temp_directory: std::env::temp_dir(),
        }
    }
}

impl CommonConfig {
    /// 验证通用配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.buffer_size < 1024 {
            return Err("缓冲区大小不能小于1024字节".to_string());
        }

        if self.buffer_size > constants::MAX_BUFFER_SIZE {
            return Err(format!(
                "缓冲区大小不能超过{}字节",
                constants::MAX_BUFFER_SIZE
            ));
        }

        if self.max_packet_size == 0 {
            return Err("最大数据包大小必须大于0".to_string());
        }

        if self.max_packet_size > constants::MAX_PACKET_SIZE {
            return Err(format!(
                "最大数据包大小不能超过{}字节",
                constants::MAX_PACKET_SIZE
            ));
        }

        if self.index_cache_size == 0 {
            return Err("索引缓存大小必须大于0".to_string());
        }

        if !self.temp_directory.exists() {
            return Err("临时目录不存在".to_string());
        }

        Ok(())
    }
}

/// 读取器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderConfig {
    /// 通用配置
    pub common: CommonConfig,
    /// 读取超时时间（毫秒）
    pub read_timeout: u64,
}

impl Default for ReaderConfig {
    fn default() -> Self {
        Self {
            common: CommonConfig::default(),
            read_timeout: 30000,
        }
    }
}

impl ReaderConfig {
    /// 验证读取器配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        // 先验证通用配置
        self.common.validate()?;

        // 验证读取器特有配置
        if self.read_timeout == 0 {
            return Err("读取超时时间必须大于0".to_string());
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
    /// 通用配置
    pub common: CommonConfig,
    /// 每个PCAP文件最大数据包数量
    pub max_packets_per_file: usize,
    /// 文件命名格式
    pub file_name_format: String,
    /// 是否启用自动刷新
    pub auto_flush: bool,
    /// 写入超时时间（毫秒）
    pub write_timeout: u64,
    /// 索引刷新间隔（毫秒）
    pub index_flush_interval: u64,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            common: CommonConfig::default(),
            max_packets_per_file: constants::DEFAULT_MAX_PACKETS_PER_FILE,
            file_name_format: constants::DEFAULT_FILE_NAME_FORMAT.to_string(),
            auto_flush: true,
            write_timeout: 30000,
            index_flush_interval: 5000,
        }
    }
}

impl WriterConfig {
    /// 验证写入器配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        // 先验证通用配置
        self.common.validate()?;

        // 验证写入器特有配置
        if self.max_packets_per_file == 0 {
            return Err("每个文件最大数据包数量必须大于0".to_string());
        }

        if self.file_name_format.is_empty() {
            return Err("文件命名格式不能为空".to_string());
        }

        if self.write_timeout == 0 {
            return Err("写入超时时间必须大于0".to_string());
        }

        if self.index_flush_interval == 0 {
            return Err("索引刷新间隔必须大于0".to_string());
        }

        Ok(())
    }

    /// 重置为默认值
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

// 为了方便访问，提供一些便捷方法
impl ReaderConfig {
    /// 创建高性能读取配置
    pub fn high_performance() -> Self {
        let mut config = Self::default();
        config.common.buffer_size = 64 * 1024; // 64KB
        config.common.index_cache_size = 5000;
        config.read_timeout = 60000; // 60秒
        config
    }

    /// 创建低内存读取配置
    pub fn low_memory() -> Self {
        let mut config = Self::default();
        config.common.buffer_size = 4096; // 4KB
        config.common.index_cache_size = 100;
        config.common.enable_index_cache = false;
        config
    }
}

impl WriterConfig {
    /// 创建高性能写入配置
    pub fn high_performance() -> Self {
        let mut config = Self::default();
        config.common.buffer_size = 64 * 1024; // 64KB
        config.max_packets_per_file = 2000;
        config.auto_flush = false;
        config.common.index_cache_size = 5000;
        config.index_flush_interval = 10000; // 10秒
        config
    }

    /// 创建低内存写入配置
    pub fn low_memory() -> Self {
        let mut config = Self::default();
        config.common.buffer_size = 4096; // 4KB
        config.max_packets_per_file = 500;
        config.auto_flush = true;
        config.common.index_cache_size = 100;
        config.common.enable_index_cache = false;
        config
    }

    /// 创建快速写入配置（适合临时数据）
    pub fn fast_write() -> Self {
        let mut config = Self::default();
        config.common.enable_validation = false;
        config.common.enable_index_cache = false;
        config.auto_flush = false;
        config.index_flush_interval = 30000; // 30秒
        config
    }
}


