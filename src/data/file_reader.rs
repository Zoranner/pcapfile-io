use log::{debug, info};
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use crate::business::config::CommonConfig;
use crate::data::models::{
    DataPacket, DataPacketHeader, PcapFileHeader,
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
    configuration: CommonConfig,
}

impl PcapFileReader {
    pub(crate) fn new(configuration: CommonConfig) -> Self {
        Self {
            file: None,
            reader: None,
            file_path: None,
            packet_count: 0,
            file_size: 0,
            header: None,
            header_position: 0,
            configuration,
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
                "文件不存在: {:?}",
                path
            )));
        }

        let file = File::open(path)
            .map_err(|e| PcapError::Io(e))?;

        let file_size = file
            .metadata()
            .map_err(|e| PcapError::Io(e))?
            .len();

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
                .map_err(|e| PcapError::Io(e))?,
        );
        self.reader = Some(reader);
        self.file_path = Some(path.to_path_buf());
        self.file_size = file_size;
        self.header = Some(header);
        self.packet_count = 0;
        self.header_position = 0;

        info!("成功打开PCAP文件: {:?}", path);
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
            .map_err(|e| PcapError::Io(e))?;

        let header =
            PcapFileHeader::from_bytes(&header_bytes)
                .map_err(|e| PcapError::InvalidFormat(e))?;

        if !header.is_valid() {
            return Err(PcapError::InvalidFormat(
                "无效的PCAP文件头".to_string(),
            ));
        }

        Ok(header)
    }

    /// 读取下一个数据包
    pub(crate) fn read_packet(
        &mut self,
    ) -> PcapResult<Option<DataPacket>> {
        let reader =
            self.reader.as_mut().ok_or_else(|| {
                PcapError::InvalidState(
                    ERR_FILE_NOT_OPEN.to_string(),
                )
            })?;

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
                .map_err(|e| PcapError::InvalidFormat(e))?;

        // 读取数据包内容
        let mut data =
            vec![0u8; header.packet_length as usize];
        reader
            .read_exact(&mut data)
            .map_err(|e| PcapError::Io(e))?;

        // 验证校验和
        if self.configuration.enable_validation {
            let calculated_checksum =
                calculate_crc32(&data);
            if calculated_checksum != header.checksum {
                return Err(PcapError::CorruptedData(format!(
                    "{}。期望: 0x{:08X}, 实际: 0x{:08X}",
                    ERR_CHECKSUM_MISMATCH, header.checksum, calculated_checksum
                )));
            }
        }

        self.packet_count += 1;

        let packet = DataPacket::new(header, data)
            .map_err(|e| PcapError::InvalidFormat(e))?;

        debug!(
            "已读取数据包，当前计数: {}",
            self.packet_count
        );
        Ok(Some(packet))
    }

    /// 关闭文件
    pub(crate) fn close(&mut self) {
        self.reader = None;
        self.file = None;
        self.file_path = None;
        self.packet_count = 0;
        self.file_size = 0;
        self.header = None;
        debug!("文件已关闭");
    }
}

impl Drop for PcapFileReader {
    fn drop(&mut self) {
        self.close();
    }
}
