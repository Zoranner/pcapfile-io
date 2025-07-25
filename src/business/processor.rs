//! 数据处理业务逻辑模块
//!
//! 实现核心的数据包处理算法、验证规则和业务流程编排。

use log::{debug, info, warn};
use std::time::SystemTime;

use crate::foundation::error::{PcapError, PcapResult};
use crate::data::models::DataPacket;
use crate::business::config::CommonConfig;

/// 数据包处理器
pub struct PacketProcessor {
    config: CommonConfig,
    processed_count: u64,
    total_bytes: u64,
    first_timestamp: Option<u64>,
    last_timestamp: Option<u64>,
}

impl PacketProcessor {
    /// 创建新的数据包处理器
    pub fn new(config: CommonConfig) -> Self {
        Self {
            config,
            processed_count: 0,
            total_bytes: 0,
            first_timestamp: None,
            last_timestamp: None,
        }
    }

    /// 处理单个数据包
    pub fn process_packet(&mut self, packet: &DataPacket) -> PcapResult<ProcessedPacket> {
        // 业务验证
        self.validate_packet(packet)?;

        // 更新统计信息
        self.update_statistics(packet);

        // 应用业务处理逻辑
        let processed = ProcessedPacket {
            original: packet.clone(),
            timestamp_ns: packet.get_timestamp_ns(),
            processed_time: SystemTime::now(),
            validation_result: ValidationResult::Valid,
        };

        debug!("已处理数据包，总计: {}", self.processed_count);
        Ok(processed)
    }

    /// 批量处理数据包
    pub fn process_batch(&mut self, packets: &[DataPacket]) -> PcapResult<Vec<ProcessedPacket>> {
        let mut results = Vec::with_capacity(packets.len());

        for packet in packets {
            match self.process_packet(packet) {
                Ok(processed) => results.push(processed),
                Err(e) => {
                    warn!("处理数据包失败: {}", e);
                    // 根据配置决定是否继续处理其他数据包
                    if !self.config.enable_validation {
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        info!("批量处理完成，成功: {}/{}", results.len(), packets.len());
        Ok(results)
    }

    /// 验证数据包
    fn validate_packet(&self, packet: &DataPacket) -> PcapResult<()> {
        if !self.config.enable_validation {
            return Ok(());
        }

        // 大小验证
        if packet.packet_length() > self.config.max_packet_size {
            return Err(PcapError::InvalidPacketSize(format!(
                "数据包大小 {} 超过限制 {}",
                packet.packet_length(),
                self.config.max_packet_size
            )));
        }

        // 完整性验证
        if !packet.is_valid() {
            return Err(PcapError::CorruptedData(
                "数据包校验和验证失败".to_string()
            ));
        }

        // 时间戳合理性验证
        let timestamp = packet.get_timestamp_ns();
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        if timestamp > now + 1_000_000_000 { // 允许1秒的时钟偏差
            return Err(PcapError::InvalidArgument(
                "数据包时间戳不合理（未来时间）".to_string()
            ));
        }

        Ok(())
    }

    /// 更新处理统计信息
    fn update_statistics(&mut self, packet: &DataPacket) {
        let timestamp = packet.get_timestamp_ns();

        // 更新时间戳范围
        match self.first_timestamp {
            None => self.first_timestamp = Some(timestamp),
            Some(first) if timestamp < first => self.first_timestamp = Some(timestamp),
            _ => {}
        }

        match self.last_timestamp {
            None => self.last_timestamp = Some(timestamp),
            Some(last) if timestamp > last => self.last_timestamp = Some(timestamp),
            _ => {}
        }

        // 更新计数和大小
        self.processed_count += 1;
        self.total_bytes += packet.total_size() as u64;
    }

    /// 获取处理统计信息
    pub fn get_statistics(&self) -> ProcessorStatistics {
        ProcessorStatistics {
            processed_count: self.processed_count,
            total_bytes: self.total_bytes,
            first_timestamp: self.first_timestamp,
            last_timestamp: self.last_timestamp,
            average_packet_size: if self.processed_count > 0 {
                self.total_bytes as f64 / self.processed_count as f64
            } else {
                0.0
            },
        }
    }

    /// 重置处理器状态
    pub fn reset(&mut self) {
        self.processed_count = 0;
        self.total_bytes = 0;
        self.first_timestamp = None;
        self.last_timestamp = None;
    }
}

/// 处理后的数据包
#[derive(Debug, Clone)]
pub struct ProcessedPacket {
    /// 原始数据包
    pub original: DataPacket,
    /// 时间戳（纳秒）
    pub timestamp_ns: u64,
    /// 处理时间
    pub processed_time: SystemTime,
    /// 验证结果
    pub validation_result: ValidationResult,
}

/// 验证结果
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// 有效
    Valid,
    /// 警告（但可以继续处理）
    Warning(String),
    /// 错误（应该停止处理）
    Error(String),
}

/// 处理器统计信息
#[derive(Debug, Clone)]
pub struct ProcessorStatistics {
    /// 已处理数据包数量
    pub processed_count: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 第一个数据包时间戳
    pub first_timestamp: Option<u64>,
    /// 最后一个数据包时间戳
    pub last_timestamp: Option<u64>,
    /// 平均数据包大小
    pub average_packet_size: f64,
}

impl ProcessorStatistics {
    /// 获取时间范围（纳秒）
    pub fn time_range_ns(&self) -> Option<u64> {
        match (self.first_timestamp, self.last_timestamp) {
            (Some(first), Some(last)) => Some(last.saturating_sub(first)),
            _ => None,
        }
    }

    /// 获取处理速率（数据包/秒）
    pub fn packet_rate(&self) -> f64 {
        if let Some(time_range) = self.time_range_ns() {
            if time_range > 0 {
                return (self.processed_count as f64) / (time_range as f64 / 1_000_000_000.0);
            }
        }
        0.0
    }

    /// 获取数据传输速率（字节/秒）
    pub fn byte_rate(&self) -> f64 {
        if let Some(time_range) = self.time_range_ns() {
            if time_range > 0 {
                return (self.total_bytes as f64) / (time_range as f64 / 1_000_000_000.0);
            }
        }
        0.0
    }
}
