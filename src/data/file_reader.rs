use log::{debug, info};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::business::config::ReaderConfig;
use crate::data::models::{
    DataPacket, DataPacketHeader, PcapFileHeader,
    ValidatedPacket,
};
use crate::foundation::error::{PcapError, PcapResult};
use crate::foundation::utils::calculate_crc32;

// 错误消息常量
const ERR_FILE_NOT_OPEN: &str = "文件未打开";
const ERR_CHECKSUM_MISMATCH: &str = "数据包校验和验证失败";

/// PCAP文件读取器
pub struct PcapFileReader {
    file: Option<File>,
    reader: Option<BufReader<File>>,
    file_path: Option<PathBuf>,
    packet_count: u64,
    file_size: u64,
    header: Option<PcapFileHeader>,
    header_position: u64,
    configuration: ReaderConfig,
    /// 当前读取位置（字节偏移）
    current_position: u64,
}

impl PcapFileReader {
    pub(crate) fn new(configuration: ReaderConfig) -> Self {
        Self {
            file: None,
            reader: None,
            file_path: None,
            packet_count: 0,
            file_size: 0,
            header: None,
            header_position: 0,
            configuration,
            current_position: 0,
        }
    }

    /// 打开PCAP文件
    pub(crate) fn open<P: AsRef<Path>>(
        &mut self,
        file_path: P,
    ) -> PcapResult<()> {
        let path = file_path.as_ref();

        if !path.exists() {
            return Err(PcapError::FileNotFound(format!(
                "文件不存在: {path:?}"
            )));
        }

        let file =
            File::open(path).map_err(PcapError::Io)?;

        let file_size =
            file.metadata().map_err(PcapError::Io)?.len();

        if file_size < PcapFileHeader::HEADER_SIZE as u64 {
            return Err(PcapError::InvalidFormat(
                "文件太小，不是有效的PCAP文件".to_string(),
            ));
        }

        let mut reader = BufReader::with_capacity(
            self.configuration.buffer_size,
            file,
        );

        // 读取并验证文件头
        let header =
            self.read_and_validate_header(&mut reader)?;

        self.file = Some(
            reader
                .get_ref()
                .try_clone()
                .map_err(PcapError::Io)?,
        );
        self.reader = Some(reader);
        self.file_path = Some(path.to_path_buf());
        self.file_size = file_size;
        self.header = Some(header);
        self.packet_count = 0;
        self.header_position = 0;
        self.current_position =
            PcapFileHeader::HEADER_SIZE as u64; // 文件头后的位置

        info!("成功打开PCAP文件: {path:?}");
        Ok(())
    }

    /// 读取并验证文件头
    fn read_and_validate_header(
        &self,
        reader: &mut BufReader<File>,
    ) -> PcapResult<PcapFileHeader> {
        let mut header_bytes =
            [0u8; PcapFileHeader::HEADER_SIZE];
        reader
            .read_exact(&mut header_bytes)
            .map_err(PcapError::Io)?;

        let header =
            PcapFileHeader::from_bytes(&header_bytes)
                .map_err(|e| {
                    PcapError::CorruptedHeader(format!(
                        "文件头解析失败: {}",
                        e
                    ))
                })?;

        if !header.is_valid() {
            return Err(PcapError::CorruptedHeader(
                "无效的PCAP文件头".to_string(),
            ));
        }

        Ok(header)
    }

    /// 读取下一个数据包
    pub(crate) fn read_packet(
        &mut self,
    ) -> PcapResult<Option<ValidatedPacket>> {
        let reader =
            self.reader.as_mut().ok_or_else(|| {
                PcapError::InvalidState(
                    ERR_FILE_NOT_OPEN.to_string(),
                )
            })?;

        // 检查是否还有足够空间读取包头
        let remaining_bytes =
            self.file_size - self.current_position;
        if remaining_bytes
            < DataPacketHeader::HEADER_SIZE as u64
        {
            return Ok(None); // 到达文件末尾
        }

        // 读取数据包头部
        let mut header_bytes =
            [0u8; DataPacketHeader::HEADER_SIZE];
        match reader.read_exact(&mut header_bytes) {
            Ok(_) => {}
            Err(ref e)
                if e.kind()
                    == io::ErrorKind::UnexpectedEof =>
            {
                return Ok(None); // 到达文件末尾
            }
            Err(e) => return Err(PcapError::Io(e)),
        }

        let header =
            DataPacketHeader::from_bytes(&header_bytes)
                .map_err(|e| {
                    PcapError::TimestampParseError {
                        message: format!(
                            "包头解析失败: {}",
                            e
                        ),
                        position: self.current_position,
                    }
                })?;

        // 检查数据包长度是否超出文件剩余空间
        let remaining_after_header = self.file_size
            - self.current_position
            - DataPacketHeader::HEADER_SIZE as u64;
        if header.packet_length as u64
            > remaining_after_header
        {
            return Err(PcapError::PacketSizeExceedsRemainingBytes {
                expected: header.packet_length,
                remaining: remaining_after_header,
                position: self.current_position + DataPacketHeader::HEADER_SIZE as u64,
            });
        }

        // 读取数据包内容
        let mut data =
            vec![0u8; header.packet_length as usize];
        reader
            .read_exact(&mut data)
            .map_err(PcapError::Io)?;

        // 验证校验和
        let calculated_checksum = calculate_crc32(&data);
        let is_valid =
            calculated_checksum == header.checksum;

        // 如果校验失败，记录警告日志
        if !is_valid {
            log::warn!(
                "{}。期望: 0x{:08X}, 实际: 0x{:08X}",
                ERR_CHECKSUM_MISMATCH,
                header.checksum,
                calculated_checksum
            );
        }

        self.packet_count += 1;
        self.current_position +=
            DataPacketHeader::HEADER_SIZE as u64
                + header.packet_length as u64;

        let packet = DataPacket::new(header, data)
            .map_err(|e| PcapError::CorruptedData {
                message: format!("数据包创建失败: {}", e),
                position: self.current_position,
            })?;

        let result = ValidatedPacket::new(packet, is_valid);

        debug!(
            "已读取数据包，当前计数: {}, 校验状态: {}, 位置: {}",
            self.packet_count,
            if is_valid { "有效" } else { "无效" },
            self.current_position
        );
        Ok(Some(result))
    }

    /// 跳转到指定字节偏移位置
    pub(crate) fn seek_to(
        &mut self,
        offset: u64,
    ) -> PcapResult<()> {
        let reader =
            self.reader.as_mut().ok_or_else(|| {
                PcapError::InvalidState(
                    "文件未打开".to_string(),
                )
            })?;

        // 跳转到指定位置
        reader
            .seek(SeekFrom::Start(offset))
            .map_err(PcapError::Io)?;

        // 更新当前位置
        self.current_position = offset;

        debug!("已跳转到位置: {}", offset);
        Ok(())
    }

    /// 在指定偏移位置读取数据包
    pub(crate) fn read_packet_at(
        &mut self,
        offset: u64,
    ) -> PcapResult<ValidatedPacket> {
        // 先跳转到指定位置
        self.seek_to(offset)?;

        // 然后读取数据包
        match self.read_packet()? {
            Some(packet) => Ok(packet),
            None => Err(PcapError::InvalidState(
                "在指定位置未找到数据包".to_string(),
            )),
        }
    }

    /// 关闭文件
    pub(crate) fn close(&mut self) {
        self.reader = None;
        self.file = None;
        self.file_path = None;
        self.packet_count = 0;
        self.file_size = 0;
        self.header = None;
        self.current_position = 0;
        debug!("文件已关闭");
    }
}

impl Drop for PcapFileReader {
    fn drop(&mut self) {
        self.close();
    }
}
