//! 格式解析和生成模块
//!
//! 负责PCAP文件格式的序列化和反序列化操作，提供底层数据格式处理功能。

use crate::foundation::error::{PcapError, PcapResult};
use crate::data::models::{DataPacket, DataPacketHeader, PcapFileHeader};

/// PCAP格式处理器
pub struct PcapFormatProcessor;

impl PcapFormatProcessor {
    /// 解析PCAP文件头
    pub fn parse_file_header(data: &[u8]) -> PcapResult<PcapFileHeader> {
        PcapFileHeader::from_bytes(data)
            .map_err(|e| PcapError::InvalidFormat(format!("解析文件头失败: {}", e)))
    }

    /// 序列化PCAP文件头
    pub fn serialize_file_header(header: &PcapFileHeader) -> Vec<u8> {
        header.to_bytes()
    }

    /// 解析数据包头
    pub fn parse_packet_header(data: &[u8]) -> PcapResult<DataPacketHeader> {
        DataPacketHeader::from_bytes(data)
            .map_err(|e| PcapError::InvalidFormat(format!("解析数据包头失败: {}", e)))
    }

    /// 序列化数据包头
    pub fn serialize_packet_header(header: &DataPacketHeader) -> Vec<u8> {
        header.to_bytes()
    }

    /// 解析完整数据包
    pub fn parse_packet(header_data: &[u8], payload_data: &[u8]) -> PcapResult<DataPacket> {
        let header = Self::parse_packet_header(header_data)?;

        if payload_data.len() != header.packet_length as usize {
            return Err(PcapError::InvalidFormat(format!(
                "数据包长度不匹配: 期望 {}, 实际 {}",
                header.packet_length,
                payload_data.len()
            )));
        }

        DataPacket::new(header, payload_data.to_vec())
            .map_err(|e| PcapError::InvalidFormat(e))
    }

    /// 序列化完整数据包
    pub fn serialize_packet(packet: &DataPacket) -> Vec<u8> {
        packet.to_bytes()
    }

    /// 验证PCAP文件格式
    pub fn validate_file_format(data: &[u8]) -> PcapResult<()> {
        if data.len() < PcapFileHeader::HEADER_SIZE {
            return Err(PcapError::InvalidFormat(
                "文件太小，不是有效的PCAP文件".to_string()
            ));
        }

        let header = Self::parse_file_header(&data[..PcapFileHeader::HEADER_SIZE])?;

        if !header.is_valid() {
            return Err(PcapError::InvalidFormat(
                "无效的PCAP文件头".to_string()
            ));
        }

        Ok(())
    }
}
