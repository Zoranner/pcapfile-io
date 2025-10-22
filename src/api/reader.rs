//! 数据集读取器模块
//!
//! 提供高级的数据集读取功能，支持多文件PCAP数据集的统一读取接口。

use log::{debug, info, warn};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

use crate::business::cache::{CacheStats, FileInfoCache};
use crate::business::config::ReaderConfig;
use crate::business::index::IndexManager;
use crate::data::file_reader::PcapFileReader;
use crate::data::models::{
    DataPacket, DatasetInfo, FileInfo, ValidatedPacket,
};
use crate::foundation::error::{PcapError, PcapResult};

// 错误消息常量
const ERROR_DATASET_NOT_FOUND: &str = "数据集目录不存在";
const ERROR_INVALID_DATASET: &str = "无效的数据集目录";

/// PCAP数据集读取器
///
/// 提供对PCAP数据集的高性能读取功能，支持：
/// - 自动索引管理和验证
/// - 顺序读取和文件切换
/// - 智能缓存和性能优化
/// - 多文件数据集统一访问
pub struct PcapReader {
    /// 数据集目录路径
    dataset_path: PathBuf,
    /// 数据集名称
    dataset_name: String,
    /// 索引管理器
    index_manager: IndexManager,
    /// 配置信息
    configuration: ReaderConfig,
    /// 当前文件读取器
    current_reader: Option<PcapFileReader>,
    /// 当前文件索引
    current_file_index: usize,
    /// 当前读取位置（全局数据包索引）
    current_position: u64,
    /// 文件信息缓存
    file_info_cache: FileInfoCache,
    /// 总大小缓存
    total_size_cache: RefCell<Option<u64>>,
    /// 是否已初始化
    is_initialized: bool,
}

impl PcapReader {
    /// 创建新的PCAP读取器
    ///
    /// # 参数
    /// - `base_path` - 基础路径
    /// - `dataset_name` - 数据集名称
    ///
    /// # 返回
    /// 返回初始化后的读取器实例
    pub fn new<P: AsRef<Path>>(
        base_path: P,
        dataset_name: &str,
    ) -> PcapResult<Self> {
        Self::new_with_config(
            base_path,
            dataset_name,
            ReaderConfig::default(),
        )
    }

    /// 创建新的PCAP读取器（带配置）
    ///
    /// # 参数
    /// - `base_path` - 基础路径
    /// - `dataset_name` - 数据集名称
    /// - `configuration` - 读取器配置信息
    ///
    /// # 返回
    /// 返回初始化后的读取器实例
    pub fn new_with_config<P: AsRef<Path>>(
        base_path: P,
        dataset_name: &str,
        configuration: ReaderConfig,
    ) -> PcapResult<Self> {
        // 验证配置有效性
        configuration.validate().map_err(|e| {
            PcapError::InvalidArgument(format!(
                "读取器配置无效: {e}"
            ))
        })?;

        let dataset_path =
            base_path.as_ref().join(dataset_name);

        // 验证数据集目录
        if !dataset_path.exists() {
            return Err(PcapError::DirectoryNotFound(
                ERROR_DATASET_NOT_FOUND.to_string(),
            ));
        }

        if !dataset_path.is_dir() {
            return Err(PcapError::InvalidArgument(
                ERROR_INVALID_DATASET.to_string(),
            ));
        }

        // 创建索引管理器
        let index_manager =
            IndexManager::new(base_path, dataset_name)?;

        // 获取缓存大小（在移动 configuration 之前）
        let cache_size = configuration.index_cache_size;

        info!("PcapReader已创建 - 数据集: {dataset_name}");

        Ok(Self {
            dataset_path,
            dataset_name: dataset_name.to_string(),
            index_manager,
            configuration,
            current_reader: None,
            current_file_index: 0,
            current_position: 0,
            file_info_cache: FileInfoCache::new(cache_size),
            total_size_cache: RefCell::new(None),
            is_initialized: false,
        })
    }

    /// 初始化读取器
    ///
    /// 确保索引可用并准备好读取操作
    pub fn initialize(&mut self) -> PcapResult<()> {
        if self.is_initialized {
            return Ok(());
        }

        info!("初始化PcapReader...");

        // 确保索引可用
        let _index = self.index_manager.ensure_index()?;

        self.is_initialized = true;
        info!("PcapReader初始化完成");
        Ok(())
    }

    /// 获取数据集信息
    pub fn get_dataset_info(
        &mut self,
    ) -> PcapResult<DatasetInfo> {
        self.initialize()?;

        let index = self
            .index_manager
            .get_index()
            .ok_or_else(|| {
                PcapError::InvalidState(
                    "索引未加载".to_string(),
                )
            })?;

        use chrono::Utc;

        Ok(DatasetInfo {
            name: self.dataset_name.clone(),
            path: self.dataset_path.clone(),
            file_count: index.data_files.files.len(),
            total_packets: index.total_packets,
            total_size: self.get_total_size()?,
            start_timestamp: if index.start_timestamp > 0 {
                Some(index.start_timestamp)
            } else {
                None
            },
            end_timestamp: if index.end_timestamp > 0 {
                Some(index.end_timestamp)
            } else {
                None
            },
            created_time: Utc::now().to_rfc3339(),
            modified_time: Utc::now().to_rfc3339(),
            has_index: true,
        })
    }

    /// 获取文件信息列表
    pub fn get_file_info_list(
        &mut self,
    ) -> PcapResult<Vec<FileInfo>> {
        self.initialize()?;

        let index = self
            .index_manager
            .get_index()
            .ok_or_else(|| {
                PcapError::InvalidState(
                    "索引未加载".to_string(),
                )
            })?;

        use chrono::Utc;
        let current_time = Utc::now().to_rfc3339();

        let mut file_infos = Vec::new();
        for file_index in &index.data_files.files {
            let file_path = self
                .dataset_path
                .join(&file_index.file_name);

            // 尝试从缓存获取文件信息
            let file_info = if let Some(cached_info) =
                self.file_info_cache.get(&file_path)
            {
                cached_info
            } else {
                // 缓存未命中，创建新的文件信息并缓存
                let file_info = FileInfo {
                    file_name: file_index.file_name.clone(),
                    file_path: file_path.clone(),
                    file_size: file_index.file_size,
                    packet_count: file_index.packet_count,
                    start_timestamp: if file_index
                        .start_timestamp
                        > 0
                    {
                        Some(file_index.start_timestamp)
                    } else {
                        None
                    },
                    end_timestamp: if file_index
                        .end_timestamp
                        > 0
                    {
                        Some(file_index.end_timestamp)
                    } else {
                        None
                    },
                    file_hash: Some(
                        file_index.file_hash.clone(),
                    ),
                    created_time: current_time.clone(),
                    modified_time: current_time.clone(),
                    is_valid: true,
                };

                // 将文件信息加入缓存
                self.file_info_cache
                    .insert(&file_path, file_info.clone());
                file_info
            };

            file_infos.push(file_info);
        }

        Ok(file_infos)
    }

    /// 获取数据集路径
    pub fn dataset_path(&self) -> &Path {
        &self.dataset_path
    }

    /// 获取数据集名称
    pub fn dataset_name(&self) -> &str {
        &self.dataset_name
    }

    /// 读取下一个数据包（默认方法，带校验结果）
    ///
    /// 从当前位置读取下一个数据包，包含校验状态信息。如果当前文件读取完毕，
    /// 会自动切换到下一个文件。
    ///
    /// # 返回
    /// - `Ok(Some(result))` - 成功读取到数据包和校验结果
    /// - `Ok(None)` - 到达文件末尾，无更多数据包
    /// - `Err(error)` - 读取过程中发生错误
    pub fn read_packet(
        &mut self,
    ) -> PcapResult<Option<ValidatedPacket>> {
        self.initialize()?;

        // 确保当前文件已打开
        self.ensure_current_file_open()?;

        loop {
            if let Some(ref mut reader) =
                self.current_reader
            {
                match reader.read_packet() {
                    Ok(Some(result)) => {
                        self.current_position += 1;
                        return Ok(Some(result));
                    }
                    Ok(None) => {
                        // 当前文件读取完毕，尝试切换到下一个文件
                        if !self.switch_to_next_file()? {
                            // 没有更多文件
                            return Ok(None);
                        }
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            } else {
                // 没有可读取的文件
                return Ok(None);
            }
        }
    }

    /// 读取下一个数据包（仅返回数据，不返回校验信息）
    ///
    /// 从当前位置读取下一个数据包，仅返回数据包本身。如果当前文件读取完毕，
    /// 会自动切换到下一个文件。
    ///
    /// # 返回
    /// - `Ok(Some(packet))` - 成功读取到数据包
    /// - `Ok(None)` - 到达文件末尾，无更多数据包
    /// - `Err(error)` - 读取过程中发生错误
    pub fn read_packet_data_only(
        &mut self,
    ) -> PcapResult<Option<DataPacket>> {
        match self.read_packet()? {
            Some(result) => Ok(Some(result.packet)),
            None => Ok(None),
        }
    }

    /// 批量读取多个数据包（默认方法，带校验结果）
    ///
    /// # 参数
    /// - `count` - 要读取的数据包数量
    ///
    /// # 返回
    pub fn read_packets(
        &mut self,
        count: usize,
    ) -> PcapResult<Vec<ValidatedPacket>> {
        self.initialize()?;

        let mut results = Vec::with_capacity(count);

        // 批量读取指定数量的数据包
        for _ in 0..count {
            if let Some(result) = self.read_packet()? {
                results.push(result);
            } else {
                break; // 没有更多数据包
            }
        }

        Ok(results)
    }

    /// 批量读取多个数据包（仅返回数据，不返回校验信息）
    ///
    /// # 参数
    /// - `count` - 要读取的数据包数量
    ///
    /// # 返回
    pub fn read_packets_data_only(
        &mut self,
        count: usize,
    ) -> PcapResult<Vec<DataPacket>> {
        self.initialize()?;

        let mut packets = Vec::with_capacity(count);

        // 批量读取指定数量的数据包
        for _ in 0..count {
            if let Some(packet) =
                self.read_packet_data_only()?
            {
                packets.push(packet);
            } else {
                break; // 没有更多数据包
            }
        }

        Ok(packets)
    }

    /// 重置读取器到数据集开始位置
    ///
    /// 将读取器重置到数据集的开始位置，后续读取将从第一个数据包开始。
    pub fn reset(&mut self) -> PcapResult<()> {
        self.initialize()?;

        // 重置当前读取位置到数据集开始
        self.current_position = 0;
        self.current_file_index = 0;

        // 关闭当前文件
        if let Some(ref mut reader) = self.current_reader {
            reader.close();
        }
        self.current_reader = None;

        // 重新打开第一个文件（如果存在）
        let index = self
            .index_manager
            .get_index()
            .ok_or_else(|| {
                PcapError::InvalidState(
                    "索引未加载".to_string(),
                )
            })?;

        if !index.data_files.files.is_empty() {
            self.open_file(0)?;
        }

        info!("读取器已重置到数据集开始位置");
        Ok(())
    }

    /// 获取索引管理器的引用
    /// 允许外部通过 reader.index().method() 的方式访问索引功能
    pub fn index(&self) -> &IndexManager {
        &self.index_manager
    }

    /// 获取索引管理器的可变引用
    /// 允许外部通过 reader.index_mut().method() 的方式访问索引功能
    pub fn index_mut(&mut self) -> &mut IndexManager {
        &mut self.index_manager
    }

    /// 按时间戳查找数据包位置
    ///
    /// # 参数
    /// - `timestamp_ns` - 目标时间戳（纳秒）
    ///
    /// # 返回
    /// 返回最接近指定时间戳的数据包索引条目，如果未找到则返回None
    pub fn seek_by_timestamp(
        &mut self,
        timestamp_ns: u64,
    ) -> PcapResult<
        Option<
            crate::business::index::types::TimestampPointer,
        >,
    > {
        self.initialize()?;

        let index = self
            .index_manager
            .get_index()
            .ok_or_else(|| {
                PcapError::InvalidState(
                    "索引未加载".to_string(),
                )
            })?;

        // 在时间戳索引中查找最接近的条目
        let mut closest_entry = None;
        let mut min_diff = u64::MAX;

        for (ts, pointer) in &index.timestamp_index {
            let diff = (*ts).abs_diff(timestamp_ns);

            if diff < min_diff {
                min_diff = diff;
                closest_entry = Some(pointer.clone());
            }
        }

        Ok(closest_entry)
    }

    /// 按时间范围读取数据包
    ///
    /// # 参数
    /// - `start_timestamp_ns` - 开始时间戳（纳秒）
    /// - `end_timestamp_ns` - 结束时间戳（纳秒）
    ///
    /// # 返回
    /// 返回指定时间范围内的所有数据包
    pub fn read_packets_by_time_range(
        &mut self,
        start_timestamp_ns: u64,
        end_timestamp_ns: u64,
    ) -> PcapResult<Vec<ValidatedPacket>> {
        self.initialize()?;

        let pointers = {
            let index = self
                .index_manager
                .get_index()
                .ok_or_else(|| {
                    PcapError::InvalidState(
                        "索引未加载".to_string(),
                    )
                })?;

            index
                .get_packets_in_range(
                    start_timestamp_ns,
                    end_timestamp_ns,
                )
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
        };

        let mut result_packets = Vec::new();
        let mut current_file_index = None;

        // 按时间顺序读取数据包
        for pointer in pointers {
            // 检查是否需要切换文件
            if current_file_index
                != Some(pointer.file_index)
            {
                self.open_file(pointer.file_index)?;
                current_file_index =
                    Some(pointer.file_index);
            }

            // 确保文件已打开
            self.ensure_current_file_open()?;

            // 读取指定位置的数据包
            let reader = self
                .current_reader
                .as_mut()
                .ok_or_else(|| {
                    PcapError::InvalidState(
                        "当前文件读取器未初始化"
                            .to_string(),
                    )
                })?;
            let packet_result = reader
                .read_packet_at(pointer.entry.byte_offset);

            match packet_result {
                Ok(packet) => {
                    // 验证时间戳是否在范围内
                    let packet_timestamp =
                        packet.packet.get_timestamp_ns();
                    if packet_timestamp
                        >= start_timestamp_ns
                        && packet_timestamp
                            <= end_timestamp_ns
                    {
                        result_packets.push(packet);
                    }
                }
                Err(e) => {
                    warn!("读取数据包失败: {}", e);
                    // 继续处理其他数据包
                }
            }
        }

        Ok(result_packets)
    }

    /// 获取缓存统计信息
    pub fn get_cache_stats(&self) -> CacheStats {
        self.file_info_cache.get_cache_stats()
    }

    /// 清理缓存
    pub fn clear_cache(&mut self) -> PcapResult<()> {
        let _ = self.file_info_cache.clear();
        debug!("缓存已清理");
        Ok(())
    }

    /// 跳转到指定时间戳（纳秒）
    ///
    /// 返回实际定位到的时间戳。如果精确匹配不存在，返回时间戳后面最接近的数据包。
    ///
    /// # 参数
    /// - `timestamp_ns` - 目标时间戳（纳秒）
    ///
    /// # 返回
    /// - `Ok(actual_timestamp)` - 成功跳转，返回实际定位到的时间戳
    /// - `Err(error)` - 未找到数据包或发生错误
    pub fn seek_to_timestamp(
        &mut self,
        timestamp_ns: u64,
    ) -> PcapResult<u64> {
        self.initialize()?;

        // 1. 先提取所需信息，避免借用冲突
        let (
            actual_ts,
            file_index,
            byte_offset,
            packet_offset,
        ) = {
            let index = self
                .index_manager
                .get_index()
                .ok_or_else(|| {
                    PcapError::InvalidState(
                        "索引未加载".to_string(),
                    )
                })?;

            // 尝试精确匹配
            let (actual_ts, pointer) = if let Some(ptr) =
                index.find_packet_by_timestamp(timestamp_ns)
            {
                (timestamp_ns, ptr.clone())
            } else {
                // 查找 >= target 的最小时间戳
                Self::find_timestamp_ge(&index.timestamp_index, timestamp_ns)
                    .ok_or_else(|| PcapError::InvalidArgument(
                        format!("未找到时间戳 >= {timestamp_ns} 的数据包")
                    ))?
            };

            // 计算文件内的序号
            let file_index_data =
                &index.data_files.files[pointer.file_index];
            let packet_offset = file_index_data
                .data_packets
                .iter()
                .position(|p| {
                    p.timestamp_ns
                        == pointer.entry.timestamp_ns
                })
                .unwrap_or(0);

            (
                actual_ts,
                pointer.file_index,
                pointer.entry.byte_offset,
                packet_offset,
            )
        };

        // 2. 打开对应文件
        self.open_file(file_index)?;

        // 3. seek 到字节偏移
        if let Some(reader) = self.current_reader.as_mut() {
            reader.seek_to(byte_offset)?;
        } else {
            return Err(PcapError::InvalidState(
                "文件未打开".to_string(),
            ));
        }

        // 4. 更新状态
        self.current_file_index = file_index;

        // 计算全局位置
        let index = self
            .index_manager
            .get_index()
            .ok_or_else(|| {
                PcapError::InvalidState(
                    "索引未加载".to_string(),
                )
            })?;
        self.current_position = self
            .calculate_global_position(
                index,
                file_index,
                packet_offset,
            );

        info!("已跳转到时间戳: {timestamp_ns}ns (实际: {actual_ts}ns), 全局位置: {}", 
            self.current_position);

        Ok(actual_ts)
    }

    /// 跳转到指定索引的数据包（从0开始）
    ///
    /// # 参数
    /// - `packet_index` - 目标数据包的全局索引（从0开始）
    pub fn seek_to_packet(
        &mut self,
        packet_index: usize,
    ) -> PcapResult<()> {
        self.initialize()?;

        // 1. 先提取所需信息，避免借用冲突
        let (target_file_idx, byte_offset, packet_offset) = {
            let index = self
                .index_manager
                .get_index()
                .ok_or_else(|| {
                    PcapError::InvalidState(
                        "索引未加载".to_string(),
                    )
                })?;

            // 检查索引范围
            if packet_index >= index.total_packets as usize
            {
                return Err(PcapError::InvalidArgument(
                    format!("数据包索引 {packet_index} 超出范围 (总数: {})", index.total_packets)
                ));
            }

            // 遍历文件，找到目标文件和文件内偏移
            let mut accumulated = 0usize;
            let mut target_file_idx = 0;
            let mut packet_offset = 0;

            for (file_idx, file) in
                index.data_files.files.iter().enumerate()
            {
                let next_accumulated = accumulated
                    + file.packet_count as usize;
                if packet_index < next_accumulated {
                    target_file_idx = file_idx;
                    packet_offset =
                        packet_index - accumulated;
                    break;
                }
                accumulated = next_accumulated;
            }

            // 获取数据包条目
            let file =
                &index.data_files.files[target_file_idx];
            let packet_entry =
                &file.data_packets[packet_offset];
            let byte_offset = packet_entry.byte_offset;

            (target_file_idx, byte_offset, packet_offset)
        };

        // 2. 打开文件并 seek
        self.open_file(target_file_idx)?;
        if let Some(reader) = self.current_reader.as_mut() {
            reader.seek_to(byte_offset)?;
        } else {
            return Err(PcapError::InvalidState(
                "文件未打开".to_string(),
            ));
        }

        // 3. 更新状态
        self.current_file_index = target_file_idx;
        self.current_position = packet_index as u64;

        info!("已跳转到数据包索引: {packet_index}, 文件: {target_file_idx}, 文件内偏移: {packet_offset}");

        Ok(())
    }

    /// 检查是否已到达文件末尾
    pub fn is_eof(&self) -> bool {
        if let Some(index) = self.index_manager.get_index()
        {
            self.current_position >= index.total_packets
        } else {
            // 未初始化时，检查是否有可读取的文件
            self.current_reader.is_none()
        }
    }

    /// 获取总数据包数量（如果索引可用）
    pub fn total_packets(&self) -> Option<usize> {
        self.index_manager
            .get_index()
            .map(|idx| idx.total_packets as usize)
    }

    /// 获取当前数据包索引位置（全局序号，从0开始）
    pub fn current_packet_index(&self) -> u64 {
        self.current_position
    }

    /// 获取当前读取进度（百分比：0.0 - 1.0）
    pub fn progress(&self) -> Option<f64> {
        self.total_packets().map(|total| {
            if total == 0 {
                1.0
            } else {
                (self.current_position as f64
                    / total as f64)
                    .min(1.0)
            }
        })
    }

    /// 跳过指定数量的数据包
    ///
    /// # 参数
    /// - `count` - 要跳过的数据包数量
    ///
    /// # 返回
    /// 实际跳过的数据包数量（可能小于请求数量，如果到达末尾）
    pub fn skip_packets(
        &mut self,
        count: usize,
    ) -> PcapResult<usize> {
        let current_idx = self.current_position as usize;
        let target_idx = current_idx + count;

        let total = self.total_packets().unwrap_or(0);
        // 限制目标索引到有效范围（0 到 total-1）
        let actual_target = if total == 0 {
            0
        } else {
            target_idx.min(total - 1)
        };
        let actual_skipped =
            actual_target.saturating_sub(current_idx);

        if actual_skipped > 0 {
            self.seek_to_packet(actual_target)?;
        }

        Ok(actual_skipped)
    }

    // =================================================================
    // 私有方法
    // =================================================================

    /// 计算指定文件索引和文件内数据包偏移对应的全局数据包位置
    fn calculate_global_position(
        &self,
        index: &crate::business::index::types::PidxIndex,
        file_index: usize,
        packet_offset_in_file: usize,
    ) -> u64 {
        let mut position = 0u64;
        for (idx, file) in
            index.data_files.files.iter().enumerate()
        {
            if idx < file_index {
                position += file.packet_count;
            } else if idx == file_index {
                position += packet_offset_in_file as u64;
                break;
            }
        }
        position
    }

    /// 查找大于等于指定时间戳的最接近时间戳及其指针
    fn find_timestamp_ge(
        timestamp_index: &std::collections::HashMap<
            u64,
            crate::business::index::types::TimestampPointer,
        >,
        target_ns: u64,
    ) -> Option<(
        u64,
        crate::business::index::types::TimestampPointer,
    )> {
        let mut candidates: Vec<u64> = timestamp_index
            .keys()
            .filter(|&&ts| ts >= target_ns)
            .copied()
            .collect();

        if candidates.is_empty() {
            return None;
        }

        candidates.sort_unstable();
        let closest_ts = candidates[0];
        timestamp_index
            .get(&closest_ts)
            .map(|ptr| (closest_ts, ptr.clone()))
    }

    /// 获取数据集总大小
    fn get_total_size(&self) -> PcapResult<u64> {
        if let Some(cached_size) =
            *self.total_size_cache.borrow()
        {
            return Ok(cached_size);
        }

        let index = self
            .index_manager
            .get_index()
            .ok_or_else(|| {
                PcapError::InvalidState(
                    "索引未加载".to_string(),
                )
            })?;

        let total_size: u64 = index
            .data_files
            .files
            .iter()
            .map(|f| f.file_size)
            .sum();

        *self.total_size_cache.borrow_mut() =
            Some(total_size);
        Ok(total_size)
    }

    /// 打开指定索引的文件
    fn open_file(
        &mut self,
        file_index: usize,
    ) -> PcapResult<()> {
        let index = self
            .index_manager
            .get_index()
            .ok_or_else(|| {
                PcapError::InvalidState(
                    "索引未加载".to_string(),
                )
            })?;

        if file_index >= index.data_files.files.len() {
            return Err(PcapError::InvalidArgument(
                format!("文件索引超出范围: {file_index}"),
            ));
        }

        // 关闭当前文件
        if let Some(ref mut reader) = self.current_reader {
            reader.close();
        }

        // 打开新文件
        let file_info = &index.data_files.files[file_index];
        let file_path =
            self.dataset_path.join(&file_info.file_name);

        let mut reader =
            crate::data::file_reader::PcapFileReader::new(
                self.configuration.clone(),
            );
        reader.open(&file_path)?;

        self.current_reader = Some(reader);
        self.current_file_index = file_index;

        debug!("已打开文件: {file_path:?}");
        Ok(())
    }

    /// 切换到下一个文件
    fn switch_to_next_file(&mut self) -> PcapResult<bool> {
        let index = self
            .index_manager
            .get_index()
            .ok_or_else(|| {
                PcapError::InvalidState(
                    "索引未加载".to_string(),
                )
            })?;

        if self.current_file_index + 1
            >= index.data_files.files.len()
        {
            // 没有更多文件
            return Ok(false);
        }

        self.open_file(self.current_file_index + 1)?;
        Ok(true)
    }

    /// 确保当前文件已打开
    fn ensure_current_file_open(
        &mut self,
    ) -> PcapResult<()> {
        if self.current_reader.is_none() {
            let index = self
                .index_manager
                .get_index()
                .ok_or_else(|| {
                    PcapError::InvalidState(
                        "索引未加载".to_string(),
                    )
                })?;

            if !index.data_files.files.is_empty() {
                self.open_file(0)?;
            }
        }
        Ok(())
    }

    /// 根据时间戳读取数据包
    pub fn read_packet_by_timestamp(
        &mut self,
        timestamp_ns: u64,
    ) -> PcapResult<Option<ValidatedPacket>> {
        let pointer = {
            let index = self
                .index_manager
                .get_index()
                .ok_or_else(|| {
                    PcapError::InvalidState(
                        "索引未加载".to_string(),
                    )
                })?;

            match index
                .find_packet_by_timestamp(timestamp_ns)
            {
                Some(ptr) => ptr.clone(),
                None => return Ok(None),
            }
        };

        // 检查是否需要切换文件
        if pointer.file_index != self.current_file_index {
            self.open_file(pointer.file_index)?;
        }

        // 确保文件已打开
        self.ensure_current_file_open()?;

        // 读取指定位置的数据包
        let reader = self
            .current_reader
            .as_mut()
            .ok_or_else(|| {
                PcapError::InvalidState(
                    "当前文件读取器未初始化".to_string(),
                )
            })?;
        let packet_result = reader
            .read_packet_at(pointer.entry.byte_offset);

        match packet_result {
            Ok(packet) => {
                // 验证时间戳是否匹配
                if packet.packet.get_timestamp_ns()
                    == timestamp_ns
                {
                    Ok(Some(packet))
                } else {
                    Err(PcapError::InvalidState(
                        "读取的数据包时间戳不匹配"
                            .to_string(),
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }
}

impl Drop for PcapReader {
    fn drop(&mut self) {
        // 关闭当前文件读取器
        if let Some(ref mut reader) = self.current_reader {
            reader.close();
        }
        debug!("PcapReader已清理");
    }
}
