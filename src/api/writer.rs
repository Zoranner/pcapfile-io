//! 数据集写入器模块
//!
//! 提供高级的数据集写入功能，支持多文件自动切换、索引生成等功能。

use log::{debug, info, warn};
use std::fs;
use std::path::{Path, PathBuf};

use crate::business::cache::{CacheStats, FileInfoCache};
use crate::business::config::WriterConfig;
use crate::business::index::IndexManager;
use crate::data::file_writer::PcapFileWriter;
use crate::data::models::{
    DataPacket, DatasetInfo, FileInfo,
};
use crate::foundation::error::{PcapError, PcapResult};
use crate::foundation::utils::DateTimeExtensions;
use chrono::Utc;

/// PCAP数据集写入器
///
/// 提供对PCAP数据集的高性能写入功能，支持：
/// - 自动文件管理和切换
/// - 智能索引生成和更新
/// - 高性能写入优化
/// - 数据完整性保证
pub struct PcapWriter {
    /// 数据集目录路径
    dataset_path: PathBuf,
    /// 数据集名称
    dataset_name: String,
    /// 索引管理器
    index_manager: IndexManager,
    /// 配置信息
    configuration: WriterConfig,
    /// 当前文件写入器
    current_writer: Option<PcapFileWriter>,
    /// 当前文件索引
    current_file_index: usize,
    /// 当前文件大小
    current_file_size: u64,
    /// 已创建的文件列表
    created_files: Vec<PathBuf>,
    /// 文件信息缓存
    file_info_cache: FileInfoCache,
    /// 缓存统计信息
    cache_stats: CacheStats,
    /// 总数据包计数
    total_packet_count: u64,
    /// 当前文件数据包计数
    current_file_packet_count: u64,
    /// 是否已初始化
    is_initialized: bool,
    /// 是否已完成
    is_finalized: bool,
}

impl PcapWriter {
    /// 创建新的PCAP写入器
    ///
    /// # 参数
    /// - `base_path` - 基础路径
    /// - `dataset_name` - 数据集名称
    ///
    /// # 返回
    /// 返回初始化后的写入器实例
    pub fn new<P: AsRef<Path>>(
        base_path: P,
        dataset_name: &str,
    ) -> PcapResult<Self> {
        Self::new_with_config(
            base_path,
            dataset_name,
            WriterConfig::default(),
        )
    }

    /// 创建新的PCAP写入器（带配置）
    ///
    /// # 参数
    /// - `base_path` - 基础路径
    /// - `dataset_name` - 数据集名称
    /// - `configuration` - 写入器配置信息
    ///
    /// # 返回
    /// 返回初始化后的写入器实例
    pub fn new_with_config<P: AsRef<Path>>(
        base_path: P,
        dataset_name: &str,
        configuration: WriterConfig,
    ) -> PcapResult<Self> {
        let dataset_path =
            base_path.as_ref().join(dataset_name);

        // 确保数据集目录存在
        if !dataset_path.exists() {
            fs::create_dir_all(&dataset_path)
                .map_err(PcapError::Io)?;
            info!("已创建数据集目录: {dataset_path:?}");
        }

        if !dataset_path.is_dir() {
            return Err(PcapError::InvalidArgument(
                format!(
                    "指定路径不是目录: {dataset_path:?}"
                ),
            ));
        }

        // 创建索引管理器（新签名：base_path + dataset_name）
        let index_manager =
            IndexManager::new(base_path, dataset_name)?;

        info!("PcapWriter已创建 - 数据集: {dataset_name}");

        Ok(Self {
            dataset_path,
            dataset_name: dataset_name.to_string(),
            index_manager,
            configuration,
            current_writer: None,
            current_file_index: 0,
            current_file_size: 0,
            created_files: Vec::new(),
            file_info_cache: FileInfoCache::new(1000),
            cache_stats: CacheStats::new(),
            total_packet_count: 0,
            current_file_packet_count: 0,
            is_initialized: false,
            is_finalized: false,
        })
    }

    /// 初始化写入器
    pub fn initialize(&mut self) -> PcapResult<()> {
        if self.is_initialized {
            return Ok(());
        }

        info!("初始化PcapWriter...");

        // 创建第一个文件
        self.create_new_file()?;

        self.is_initialized = true;
        info!("PcapWriter初始化完成");
        Ok(())
    }

    /// 完成写入并生成索引
    pub fn finalize(&mut self) -> PcapResult<()> {
        if self.is_finalized {
            return Ok(());
        }

        info!("正在完成PcapWriter...");

        // 刷新并关闭当前文件
        if let Some(ref mut writer) = self.current_writer {
            writer.flush()?;
            writer.close();
        }
        self.current_writer = None;

        // 如果启用索引缓存，生成索引
        if self.configuration.common.enable_index_cache {
            self.index_manager.regenerate_index()?;
        }

        self.is_finalized = true;
        info!(
            "PcapWriter已完成 - 总文件数: {}, 总数据包数: {}",
            self.created_files.len(),
            self.total_packet_count
        );

        Ok(())
    }

    /// 获取数据集信息
    pub fn get_dataset_info(&self) -> DatasetInfo {
        use chrono::Utc;

        DatasetInfo {
            name: self.dataset_name.clone(),
            path: self.dataset_path.clone(),
            file_count: self.created_files.len(),
            total_packets: self.total_packet_count,
            total_size: self.get_total_size(),
            start_timestamp: None, // 需要从实际数据中计算
            end_timestamp: None,   // 需要从实际数据中计算
            created_time: Utc::now().to_rfc3339(),
            modified_time: Utc::now().to_rfc3339(),
            has_index: self
                .configuration
                .common
                .enable_index_cache,
        }
    }

    /// 获取文件信息列表
    pub fn get_file_info_list(&self) -> Vec<FileInfo> {
        let mut file_infos = Vec::new();

        use chrono::Utc;
        let current_time = Utc::now().to_rfc3339();

        for file_path in &self.created_files {
            if let Ok(metadata) = fs::metadata(file_path) {
                let file_info = FileInfo {
                    file_name: file_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("")
                        .to_string(),
                    file_path: file_path.clone(),
                    file_size: metadata.len(),
                    packet_count: 0, // 需要从索引中获取
                    start_timestamp: None,
                    end_timestamp: None,
                    file_hash: None,
                    created_time: current_time.clone(),
                    modified_time: current_time.clone(),
                    is_valid: true,
                };
                file_infos.push(file_info);
            }
        }

        file_infos
    }

    /// 获取索引管理器的引用
    /// 允许外部通过 writer.index().method() 的方式访问索引功能
    pub fn index(&self) -> &IndexManager {
        &self.index_manager
    }

    /// 获取索引管理器的可变引用
    /// 允许外部通过 writer.index_mut().method() 的方式访问索引功能
    pub fn index_mut(&mut self) -> &mut IndexManager {
        &mut self.index_manager
    }

    /// 获取数据集路径
    pub fn dataset_path(&self) -> &Path {
        &self.dataset_path
    }

    /// 获取数据集名称
    pub fn dataset_name(&self) -> &str {
        &self.dataset_name
    }

    /// 写入单个数据包
    ///
    /// # 参数
    /// - `packet` - 要写入的数据包
    ///
    /// # 返回
    /// - `Ok(())` - 成功写入数据包
    /// - `Err(error)` - 写入过程中发生错误
    pub fn write_packet(
        &mut self,
        packet: &DataPacket,
    ) -> PcapResult<()> {
        if self.is_finalized {
            return Err(PcapError::InvalidState(
                "写入器已完成，无法继续写入".to_string(),
            ));
        }

        // 确保初始化
        if !self.is_initialized {
            self.initialize()?;
        }

        // 检查是否需要切换文件
        if self.should_switch_file() {
            self.switch_to_new_file()?;
        }

        // 写入数据包
        if let Some(ref mut writer) = self.current_writer {
            writer.write_packet(packet)?;

            // 更新统计信息
            self.current_file_size +=
                packet.packet_length() as u64 + 16; // 16字节包头
            self.current_file_packet_count += 1;
            self.total_packet_count += 1;

            debug!(
                "已写入数据包，当前文件大小: {} 字节",
                self.current_file_size
            );
        } else {
            return Err(PcapError::InvalidState(
                "没有可用的写入器".to_string(),
            ));
        }

        Ok(())
    }

    /// 批量写入多个数据包
    ///
    /// # 参数
    /// - `packets` - 要写入的数据包列表
    ///
    /// # 返回
    pub fn write_packets(
        &mut self,
        packets: &[DataPacket],
    ) -> PcapResult<()> {
        for packet in packets {
            self.write_packet(packet)?;
        }
        Ok(())
    }

    /// 刷新当前文件
    ///
    /// 将当前文件的缓冲区数据写入磁盘，确保数据完整性。
    pub fn flush(&mut self) -> PcapResult<()> {
        if let Some(ref mut writer) = self.current_writer {
            writer.flush()?;
            debug!("缓冲区已刷新");
        }
        Ok(())
    }

    /// 获取缓存统计信息
    pub fn get_cache_stats(&self) -> &CacheStats {
        &self.cache_stats
    }

    /// 清理缓存
    pub fn clear_cache(&mut self) -> PcapResult<()> {
        let _ = self.file_info_cache.clear();
        self.cache_stats = CacheStats::new();
        debug!("缓存已清理");
        Ok(())
    }

    // =================================================================
    // 私有方法
    // =================================================================

    /// 创建新的PCAP文件
    fn create_new_file(&mut self) -> PcapResult<()> {
        // 生成符合标准的文件名: data_yyMMdd_HHmmss_nnnnnnnnn.pcap
        let time_str = Utc::now().to_filename_string();
        let filename = format!("data_{time_str}.pcap");

        let file_path = self.dataset_path.join(&filename);

        // 创建新的写入器
        let mut writer = PcapFileWriter::new(
            self.configuration.common.clone(),
            self.configuration.auto_flush,
        );
        writer
            .create(&self.dataset_path, &filename)
            .map_err(PcapError::InvalidFormat)?;

        // 关闭之前的写入器
        if let Some(ref mut old_writer) =
            self.current_writer
        {
            old_writer
                .flush()
                .map_err(PcapError::InvalidFormat)?;
            old_writer.close();
        }

        // 更新状态
        self.current_writer = Some(writer);
        self.current_file_size = 0;
        self.current_file_packet_count = 0;
        self.created_files.push(file_path.clone());

        info!("已创建新文件: {file_path:?}");
        Ok(())
    }

    /// 检查是否需要切换文件
    fn should_switch_file(&self) -> bool {
        // 检查数据包数量限制
        if self.current_file_packet_count
            >= self.configuration.max_packets_per_file
                as u64
        {
            return true;
        }

        false
    }

    /// 切换到新文件
    fn switch_to_new_file(&mut self) -> PcapResult<()> {
        self.current_file_index += 1;
        self.create_new_file()
    }

    /// 获取总大小
    fn get_total_size(&self) -> u64 {
        self.created_files
            .iter()
            .map(|path| {
                fs::metadata(path)
                    .map(|metadata| metadata.len())
                    .unwrap_or(0)
            })
            .sum()
    }
}

impl Drop for PcapWriter {
    fn drop(&mut self) {
        if !self.is_finalized {
            if let Err(e) = self.finalize() {
                warn!("完成PcapWriter时出错: {e}");
            }
        }
    }
}
