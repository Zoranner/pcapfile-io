use log::info;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::business::config::CommonConfig;
use crate::data::models::{DataPacket, PcapFileHeader};

/// PCAP文件写入器
pub struct PcapFileWriter {
    file: Option<File>,
    writer: Option<BufWriter<File>>,
    file_path: Option<PathBuf>,
    packet_count: u64,
    total_size: u64,
    max_packets_per_file: usize,
    auto_flush: bool,
    configuration: CommonConfig,
}

impl PcapFileWriter {
    pub(crate) fn new(configuration: CommonConfig, max_packets_per_file: usize, auto_flush: bool) -> Self {
        Self {
            file: None,
            writer: None,
            file_path: None,
            packet_count: 0,
            total_size: 0,
            max_packets_per_file,
            auto_flush,
            configuration,
        }
    }

    /// 创建新的PCAP文件
    pub(crate) fn create<P: AsRef<Path>>(&mut self, file_path: P) -> Result<(), String> {
        let path = file_path.as_ref();

        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(path)
            .map_err(|e| format!("创建文件失败: {:?}, 错误: {}", path, e))?;

        let mut writer = BufWriter::with_capacity(self.configuration.buffer_size, file);

        // 写入文件头
        let header = PcapFileHeader::new(0);
        writer
            .write_all(&header.to_bytes())
            .map_err(|e| format!("写入文件头失败: {}", e))?;

        if self.auto_flush {
            writer
                .flush()
                .map_err(|e| format!("刷新缓冲区失败: {}", e))?;
        }

        self.file = Some(
            writer
                .get_ref()
                .try_clone()
                .map_err(|e| format!("无法克隆文件句柄: {}", e))?,
        );
        self.writer = Some(writer);
        self.file_path = Some(path.to_path_buf());
        self.packet_count = 0;
        self.total_size = PcapFileHeader::HEADER_SIZE as u64;

        info!("成功创建PCAP文件: {:?}", path);
        Ok(())
    }

    /// 写入数据包
    pub(crate) fn write_packet(&mut self, packet: &DataPacket) -> Result<u64, String> {
        // 检查是否需要创建新文件
        if self.packet_count >= self.max_packets_per_file as u64 {
            self.create_new_file()?;
        }

        let writer = self.writer.as_mut().ok_or("文件未打开")?;

        // 获取当前位置作为偏移量
        let offset = self.total_size;

        // 写入数据包
        let packet_bytes = packet.to_bytes();
        writer
            .write_all(&packet_bytes)
            .map_err(|e| format!("写入数据包失败: {}", e))?;

        self.packet_count += 1;
        self.total_size += packet_bytes.len() as u64;

        if self.auto_flush {
            writer
                .flush()
                .map_err(|e| format!("刷新缓冲区失败: {}", e))?;
        }

        Ok(offset)
    }

    /// 创建新文件
    fn create_new_file(&mut self) -> Result<(), String> {
        let current_path = self.file_path.clone();
        if let Some(path) = current_path {
            // 关闭当前文件
            self.close();

            // 生成新文件名
            let new_path = self.generate_new_file_path(&path)?;

            // 创建新文件
            self.create(new_path)?;
        }
        Ok(())
    }

    /// 生成新文件路径
    fn generate_new_file_path(&self, current_path: &Path) -> Result<PathBuf, String> {
        let stem = current_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("无法获取文件名")?;

        let extension = current_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("pcap");

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "获取时间戳失败")?
            .as_nanos();

        let new_filename = format!("{}_{}.{}", stem, timestamp, extension);
        Ok(current_path.with_file_name(new_filename))
    }

    /// 刷新缓冲区
    pub(crate) fn flush(&mut self) -> Result<(), String> {
        if let Some(writer) = &mut self.writer {
            writer
                .flush()
                .map_err(|e| format!("刷新缓冲区失败: {}", e))?;
        }
        Ok(())
    }

    /// 关闭文件
    pub(crate) fn close(&mut self) {
        if let Some(writer) = &mut self.writer {
            let _ = writer.flush();
        }
        self.writer = None;
        self.file = None;
        self.file_path = None;
        self.packet_count = 0;
        self.total_size = 0;
    }

    /// 获取当前文件路径（内部使用）
    pub(crate) fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// 获取已写入的数据包数量（内部使用）
    pub(crate) fn packet_count(&self) -> u64 {
        self.packet_count
    }

    /// 获取总大小（内部使用）
    pub(crate) fn total_size(&self) -> u64 {
        self.total_size
    }
}

impl Drop for PcapFileWriter {
    fn drop(&mut self) {
        self.close();
    }
}
