use log::{debug, info, warn};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::foundation::error::{PcapError, PcapResult};
use crate::business::index::types::{PacketIndexEntry, PcapFileIndex, PidxIndex};
use crate::business::config::CommonConfig;
use crate::data::file_reader::PcapFileReader;

/// PIDX索引管理器
///
/// 提供索引的完整生命周期管理，包括：
/// - 自动加载和验证现有索引
/// - 检测数据变化并重建索引
/// - 生成和保存新索引
///
/// 这是一个统一的入口，封装了用户的典型使用流程：
/// 加载索引 -> 验证有效性 -> 重新生成（如果需要）
pub struct IndexManager {
    /// 数据集目录路径
    dataset_path: PathBuf,
    /// 数据集名称
    dataset_name: String,
    /// 当前索引
    index: Option<PidxIndex>,
}

impl IndexManager {
    /// 创建新的索引管理器
    ///
    /// # 参数
    /// - `dataset_path` - 数据集目录路径
    ///
    /// # 返回
    /// 返回初始化后的管理器实例
    pub fn new<P: AsRef<Path>>(dataset_path: P) -> PcapResult<Self> {
        let path = dataset_path.as_ref().to_path_buf();

        if !path.exists() {
            return Err(PcapError::DirectoryNotFound(format!(
                "数据集目录不存在: {:?}",
                path
            )));
        }

        if !path.is_dir() {
            return Err(PcapError::InvalidArgument(format!(
                "指定路径不是目录: {:?}",
                path
            )));
        }

        let dataset_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("未命名数据集")
            .to_string();

        Ok(Self {
            dataset_path: path,
            dataset_name,
            index: None,
        })
    }

    /// 确保索引可用
    ///
    /// 这是主要的入口方法，实现了完整的索引管理流程：
    /// 1. 尝试加载现有索引
    /// 2. 验证索引有效性
    /// 3. 如果无效则重新生成
    /// 4. 返回可用的索引
    pub fn ensure_index(&mut self) -> PcapResult<&PidxIndex> {
        info!("正在检查数据集索引: {}", self.dataset_name);

        // 1. 尝试加载现有索引
        if let Some(pidx_path) = self.find_pidx_file()? {
            info!("找到索引文件: {:?}", pidx_path);

            // 验证格式并加载
            if self.validate_pidx_format(&pidx_path)? {
                match self.load_index(&pidx_path) {
                    Ok(index) => {
                        // 验证索引有效性
                        if self.is_index_valid(&index)? {
                            info!("使用现有的有效索引文件");
                            self.index = Some(index);
                            return Ok(self.index.as_ref().unwrap());
                        } else {
                            info!("索引文件无效或过时，需要重新生成");
                        }
                    }
                    Err(e) => {
                        warn!("加载索引文件失败: {}, 将重新生成", e);
                    }
                }
            } else {
                warn!("索引文件格式无效，将重新生成");
            }
        } else {
            info!("未找到索引文件，将自动生成");
        }

        // 2. 生成新索引
        self.generate_index()?;
        Ok(self.index.as_ref().unwrap())
    }

    /// 强制重新生成索引
    pub fn regenerate_index(&mut self) -> PcapResult<PathBuf> {
        self.index = None;
        self.generate_index()
    }

    /// 获取当前索引的引用
    pub fn get_index(&self) -> Option<&PidxIndex> {
        self.index.as_ref()
    }

    /// 验证索引是否需要重建
    pub fn needs_rebuild(&self) -> PcapResult<bool> {
        if let Some(index) = &self.index {
            let current_files = self.scan_pcap_files()?;

            // 检查文件数量是否匹配
            if current_files.len() != index.data_files.files.len() {
                return Ok(true);
            }

            // 检查每个文件的哈希值
            for file_index in &index.data_files.files {
                if let Some(current_file) = current_files
                    .iter()
                    .find(|f| f.file_name().and_then(|n| n.to_str()) == Some(&file_index.file_name))
                {
                    match self.calculate_file_hash(current_file) {
                        Ok(hash) => {
                            if hash != file_index.file_hash {
                                return Ok(true);
                            }
                        }
                        Err(_) => return Ok(true),
                    }
                } else {
                    return Ok(true);
                }
            }

            Ok(false)
        } else {
            Ok(true) // 没有索引就需要重建
        }
    }

    /// 异步验证索引文件的有效性
    pub async fn verify_index_validity(&self) -> PcapResult<bool> {
        if let Some(index) = &self.index {
            info!("验证索引文件有效性...");

            for file_index in &index.data_files.files {
                let file_path = self.dataset_path.join(&file_index.file_name);

                if !file_path.exists() {
                    warn!("PCAP文件不存在: {:?}", file_path);
                    return Ok(false);
                }

                // 验证文件哈希
                match self.verify_file_hash(&file_path, &file_index.file_hash) {
                    Ok(true) => {
                        debug!("文件哈希验证通过: {}", file_index.file_name);
                    }
                    Ok(false) => {
                        warn!("文件哈希验证失败: {}", file_index.file_name);
                        return Ok(false);
                    }
                    Err(e) => {
                        warn!("计算文件哈希失败: {}, 错误: {}", file_index.file_name, e);
                        return Ok(false);
                    }
                }
            }

            info!("索引文件验证通过");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // =================================================================
    // 私有方法 - 索引生成相关
    // =================================================================

    /// 生成并保存数据集的时间索引
    fn generate_index(&mut self) -> PcapResult<PathBuf> {
        info!("开始生成数据集时间索引: {}", self.dataset_name);

        let mut index = PidxIndex::new(Some(format!("数据集: {}", self.dataset_name)));

        // 扫描目录中的所有PCAP文件
        let pcap_files = self.scan_pcap_files()?;

        if pcap_files.is_empty() {
            info!("数据集目录中未找到PCAP文件，生成空索引结构");

            // 对于空数据集，创建基础的空索引结构
            index.start_timestamp = 0;
            index.end_timestamp = 0;
            index.total_packets = 0;
            index.total_duration = 0;

            // 保存空索引到文件
            self.index = Some(index);
            let pidx_file_path = self.get_pidx_file_path();
            self.save_index_to_file(&pidx_file_path)?;

            info!("空索引文件已生成: {:?}", pidx_file_path);
            return Ok(pidx_file_path);
        }

        info!("找到 {} 个PCAP文件，开始分析...", pcap_files.len());

        let mut global_start_timestamp = u64::MAX;
        let mut global_end_timestamp = 0u64;
        let mut timestamp_index = HashMap::new();

        // 分析每个PCAP文件
        for file_path in pcap_files {
            match self.index_pcap_file(&file_path) {
                Ok(file_index) => {
                    // 更新全局时间戳
                    if file_index.start_timestamp < global_start_timestamp {
                        global_start_timestamp = file_index.start_timestamp;
                    }
                    if file_index.end_timestamp > global_end_timestamp {
                        global_end_timestamp = file_index.end_timestamp;
                    }

                    // 构建时间戳索引
                    for packet in &file_index.data_packets {
                        timestamp_index.insert(packet.timestamp_ns, packet.clone());
                    }

                    index.data_files.files.push(file_index);
                }
                Err(e) => {
                    warn!("分析PCAP文件失败: {:?}, 错误: {}", file_path, e);
                    // 继续处理其他文件
                }
            }
        }

        // 设置全局时间信息
        if global_start_timestamp != u64::MAX {
            index.start_timestamp = global_start_timestamp;
            index.end_timestamp = global_end_timestamp;
        } else {
            // 如果所有文件都分析失败，设置默认值
            index.start_timestamp = 0;
            index.end_timestamp = 0;
        }

        // 更新统计信息
        index.update_time_range();
        index.update_total_packets();
        index.timestamp_index = timestamp_index;

        // 保存索引
        self.index = Some(index);
        let pidx_file_path = self.get_pidx_file_path();
        self.save_index_to_file(&pidx_file_path)?;

        info!(
            "索引生成完成 - 文件数: {}, 总数据包: {}, 时长: {:.2}秒",
            self.index.as_ref().unwrap().data_files.files.len(),
            self.index.as_ref().unwrap().total_packets,
            (self.index.as_ref().unwrap().end_timestamp
                - self.index.as_ref().unwrap().start_timestamp) as f64
                / 1_000_000_000.0
        );

        info!("PIDX索引文件已保存: {:?}", pidx_file_path);
        Ok(pidx_file_path)
    }

    /// 为单个PCAP文件生成索引
    fn index_pcap_file<P: AsRef<Path>>(&self, file_path: P) -> PcapResult<PcapFileIndex> {
        let path = file_path.as_ref();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        debug!("正在分析PCAP文件: {}", file_name);

        // 计算文件哈希
        let file_hash = self.calculate_file_hash(path)?;
        let file_size = fs::metadata(path).map_err(|e| PcapError::Io(e))?.len();

        // 打开PCAP文件并读取所有数据包
        let mut reader = PcapFileReader::new(CommonConfig::default());
        reader.open(path)?;
        let mut packets = Vec::new();
        let mut packet_count = 0u64;
        let mut current_position = 16u64; // PCAP文件头后的位置

        let mut start_timestamp = u64::MAX;
        let mut end_timestamp = 0u64;

        // 读取所有数据包并记录位置
        while let Some(packet) = reader.read_packet()? {
            let timestamp_ns = packet.get_timestamp_ns();

            // 更新时间范围
            if timestamp_ns < start_timestamp {
                start_timestamp = timestamp_ns;
            }
            if timestamp_ns > end_timestamp {
                end_timestamp = timestamp_ns;
            }

            // 创建索引条目
            let index_entry = PacketIndexEntry {
                timestamp_ns,
                byte_offset: current_position,
                packet_size: packet.packet_length() as u32,
            };

            packets.push(index_entry);
            packet_count += 1;

            // 更新当前位置（16字节包头 + 数据内容）
            current_position += 16 + packet.packet_length() as u64;
        }

        let file_index = PcapFileIndex {
            file_name,
            file_hash,
            file_size,
            packet_count,
            start_timestamp,
            end_timestamp,
            data_packets: packets,
        };

        debug!(
            "文件分析完成: {} 个数据包, 时间范围: {}ns - {}ns",
            packet_count, start_timestamp, end_timestamp
        );

        Ok(file_index)
    }

    // =================================================================
    // 私有方法 - 索引读取和验证相关
    // =================================================================

    /// 从PIDX文件加载索引
    fn load_index<P: AsRef<Path>>(&self, pidx_file_path: P) -> PcapResult<PidxIndex> {
        let xml_content =
            fs::read_to_string(pidx_file_path.as_ref()).map_err(|e| PcapError::Io(e))?;

        let mut index = self.deserialize_from_xml(&xml_content)?;
        index.build_timestamp_index();

        info!("PIDX索引文件已加载: {:?}", pidx_file_path.as_ref());
        Ok(index)
    }

    /// 从数据集目录查找PIDX文件
    fn find_pidx_file(&self) -> PcapResult<Option<PathBuf>> {
        let entries = fs::read_dir(&self.dataset_path).map_err(|e| PcapError::Io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| PcapError::Io(e))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension.to_str() == Some("pidx") {
                        return Ok(Some(path));
                    }
                }
            }
        }

        Ok(None)
    }

    /// 验证索引是否有效
    fn is_index_valid(&self, index: &PidxIndex) -> PcapResult<bool> {
        // 检查是否需要重建
        let current_files = self.scan_pcap_files()?;

        // 检查文件数量是否匹配
        if current_files.len() != index.data_files.files.len() {
            return Ok(false);
        }

        // 检查每个文件的哈希值
        for file_index in &index.data_files.files {
            if let Some(current_file) = current_files
                .iter()
                .find(|f| f.file_name().and_then(|n| n.to_str()) == Some(&file_index.file_name))
            {
                match self.calculate_file_hash(current_file) {
                    Ok(hash) => {
                        if hash != file_index.file_hash {
                            return Ok(false);
                        }
                    }
                    Err(_) => return Ok(false),
                }
            } else {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// 快速验证PIDX文件格式
    fn validate_pidx_format<P: AsRef<Path>>(&self, pidx_file_path: P) -> PcapResult<bool> {
        let xml_content =
            fs::read_to_string(pidx_file_path.as_ref()).map_err(|e| PcapError::Io(e))?;

        match self.deserialize_from_xml(&xml_content) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    // =================================================================
    // 私有方法 - 工具函数
    // =================================================================

    /// 扫描目录中的PCAP文件
    fn scan_pcap_files(&self) -> PcapResult<Vec<PathBuf>> {
        let mut pcap_files = Vec::new();
        let entries = fs::read_dir(&self.dataset_path).map_err(|e| PcapError::Io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| PcapError::Io(e))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension.to_str() == Some("pcap") {
                        pcap_files.push(path);
                    }
                }
            }
        }

        // 按文件名排序
        pcap_files.sort();
        Ok(pcap_files)
    }

    /// 计算文件的SHA256哈希值
    fn calculate_file_hash<P: AsRef<Path>>(&self, file_path: P) -> PcapResult<String> {
        let file = File::open(file_path.as_ref()).map_err(|e| PcapError::Io(e))?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = reader.read(&mut buffer).map_err(|e| PcapError::Io(e))?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// 验证PCAP文件是否与索引中的哈希值匹配
    fn verify_file_hash<P: AsRef<Path>>(&self, file_path: P, expected_hash: &str) -> PcapResult<bool> {
        let actual_hash = self.calculate_file_hash(file_path)?;
        Ok(actual_hash == expected_hash)
    }

    /// 从XML格式反序列化索引
    fn deserialize_from_xml(&self, xml_content: &str) -> PcapResult<PidxIndex> {
        let mut index: PidxIndex = serde_xml_rs::from_str(xml_content)
            .map_err(|e| PcapError::InvalidFormat(format!("XML反序列化失败: {}", e)))?;
        index.build_timestamp_index();
        Ok(index)
    }

    /// 将索引序列化为XML格式
    fn serialize_to_xml(&self, index: &PidxIndex) -> PcapResult<String> {
        let xml_content = serde_xml_rs::to_string(index)
            .map_err(|e| PcapError::InvalidFormat(format!("XML序列化失败: {}", e)))?;
        Ok(xml_content)
    }

    /// 保存索引到文件
    fn save_index_to_file(&self, pidx_file_path: &PathBuf) -> PcapResult<()> {
        if let Some(index) = &self.index {
            let xml_content = self.serialize_to_xml(index)?;
            fs::write(pidx_file_path, xml_content).map_err(|e| PcapError::Io(e))?;
        }
        Ok(())
    }

    /// 获取PIDX文件路径
    fn get_pidx_file_path(&self) -> PathBuf {
        let pidx_filename = format!("{}.pidx", self.dataset_name);
        self.dataset_path.join(pidx_filename)
    }
}
