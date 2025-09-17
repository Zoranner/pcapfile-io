use log::info;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::business::config::WriterConfig;
use crate::data::models::{DataPacket, PcapFileHeader};

/// PCAP文件写入器
pub struct PcapFileWriter {
    file: Option<File>,
    writer: Option<BufWriter<File>>,
    file_path: Option<PathBuf>,
    packet_count: u64,
    total_size: u64,
    configuration: WriterConfig,
}

impl PcapFileWriter {
    pub(crate) fn new(configuration: WriterConfig) -> Self {
        Self {
            file: None,
            writer: None,
            file_path: None,
            packet_count: 0,
            total_size: 0,
            configuration,
        }
    }

    /// 创建新的PCAP文件
    pub(crate) fn create<P: AsRef<Path>>(
        &mut self,
        base_dir: P,
        filename: &str,
    ) -> Result<(), String> {
        let path = base_dir.as_ref().join(filename);

        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(
                |e| format!("创建目录失败: {e}"),
            )?;
        }

        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .open(&path)
            .map_err(|e| {
                format!("创建文件失败: {path:?}, 错误: {e}")
            })?;

        let mut writer = BufWriter::with_capacity(
            self.configuration.buffer_size,
            file,
        );

        // 写入文件头
        let header = PcapFileHeader::new(0);
        writer
            .write_all(&header.to_bytes())
            .map_err(|e| format!("写入文件头失败: {e}"))?;

        if self.configuration.auto_flush {
            writer.flush().map_err(|e| {
                format!("刷新缓冲区失败: {e}")
            })?;
        }

        self.file =
            Some(writer.get_ref().try_clone().map_err(
                |e| format!("无法克隆文件句柄: {e}"),
            )?);
        self.writer = Some(writer);
        self.file_path = Some(path.to_path_buf());
        self.packet_count = 0;
        self.total_size =
            PcapFileHeader::HEADER_SIZE as u64;

        info!("成功创建PCAP文件: {path:?}");
        Ok(())
    }

    /// 写入数据包
    pub(crate) fn write_packet(
        &mut self,
        packet: &DataPacket,
    ) -> Result<u64, String> {
        let writer =
            self.writer.as_mut().ok_or("文件未打开")?;

        // 获取当前位置作为偏移量
        let offset = self.total_size;

        // 写入数据包
        let packet_bytes = packet.to_bytes();
        writer
            .write_all(&packet_bytes)
            .map_err(|e| format!("写入数据包失败: {e}"))?;

        self.packet_count += 1;
        self.total_size += packet_bytes.len() as u64;

        if self.configuration.auto_flush {
            writer.flush().map_err(|e| {
                format!("刷新缓冲区失败: {e}")
            })?;
        }

        Ok(offset)
    }

    /// 刷新缓冲区
    pub(crate) fn flush(&mut self) -> Result<(), String> {
        if let Some(writer) = &mut self.writer {
            writer.flush().map_err(|e| {
                format!("刷新缓冲区失败: {e}")
            })?;
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
}

impl Drop for PcapFileWriter {
    fn drop(&mut self) {
        self.close();
    }
}
