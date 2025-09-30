use std::collections::HashMap;

// 索引相关结构体和实现，从 structures.rs 移动而来
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "packet")]
pub struct PacketIndexEntry {
    #[serde(rename = "@timestamp_ns")]
    pub timestamp_ns: u64,
    #[serde(rename = "@byte_offset")]
    pub byte_offset: u64,
    #[serde(rename = "@packet_size")]
    pub packet_size: u32,
}

/// 时间戳指针结构（仅用于内存索引，不参与序列化）
#[derive(Debug, Clone)]
pub struct TimestampPointer {
    /// 文件在 data_files.files 中的索引
    pub file_index: usize,
    /// 数据包索引条目
    pub entry: PacketIndexEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "file")]
pub struct PcapFileIndex {
    #[serde(rename = "@name")]
    pub file_name: String,
    #[serde(rename = "@hash")]
    pub file_hash: String,
    #[serde(rename = "@size")]
    pub file_size: u64,
    #[serde(rename = "@packet_count")]
    pub packet_count: u64,
    #[serde(rename = "@start_timestamp")]
    pub start_timestamp: u64,
    #[serde(rename = "@end_timestamp")]
    pub end_timestamp: u64,
    #[serde(rename = "packet", default)]
    pub data_packets: Vec<PacketIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "index")]
pub struct PidxIndex {
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "created_time")]
    pub created_time: String,
    #[serde(rename = "start_timestamp")]
    pub start_timestamp: u64,
    #[serde(rename = "end_timestamp")]
    pub end_timestamp: u64,
    #[serde(rename = "total_packets")]
    pub total_packets: u64,
    #[serde(rename = "total_duration")]
    pub total_duration: u64,
    #[serde(rename = "data_files")]
    pub data_files: DataFiles,
    #[serde(skip)]
    pub timestamp_index: HashMap<u64, TimestampPointer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFiles {
    #[serde(rename = "file", default)]
    pub files: Vec<PcapFileIndex>,
}

impl PidxIndex {
    pub fn new(description: Option<String>) -> Self {
        use chrono::Utc;
        Self {
            description: description.unwrap_or_default(),
            created_time: Utc::now().to_rfc3339(),
            start_timestamp: 0,
            end_timestamp: 0,
            total_packets: 0,
            total_duration: 0,
            data_files: DataFiles { files: Vec::new() },
            timestamp_index: HashMap::new(),
        }
    }
    pub fn update_time_range(&mut self) {
        if self.data_files.files.is_empty() {
            self.start_timestamp = 0;
            self.end_timestamp = 0;
            self.total_duration = 0;
            return;
        }
        self.start_timestamp = self
            .data_files
            .files
            .iter()
            .map(|f| f.start_timestamp)
            .min()
            .unwrap_or(0);
        self.end_timestamp = self
            .data_files
            .files
            .iter()
            .map(|f| f.end_timestamp)
            .max()
            .unwrap_or(0);
        self.total_duration = self
            .end_timestamp
            .saturating_sub(self.start_timestamp);
    }

    pub fn update_total_packets(&mut self) {
        self.total_packets = self
            .data_files
            .files
            .iter()
            .map(|f| f.packet_count)
            .sum();
    }

    pub fn build_timestamp_index(&mut self) {
        self.timestamp_index.clear();
        for (file_idx, file_index) in self.data_files.files.iter().enumerate() {
            for packet in &file_index.data_packets {
                let pointer = TimestampPointer {
                    file_index: file_idx,
                    entry: packet.clone(),
                };
                self.timestamp_index.insert(
                    packet.timestamp_ns,
                    pointer,
                );
            }
        }
        log::debug!(
            "构建时间戳索引完成，包含 {} 条目",
            self.timestamp_index.len()
        );
    }

    pub fn find_packet_by_timestamp(
        &self,
        timestamp_ns: u64,
    ) -> Option<&TimestampPointer> {
        self.timestamp_index.get(&timestamp_ns)
    }

    pub fn get_packets_in_range(
        &self,
        start_ns: u64,
        end_ns: u64,
    ) -> Vec<&TimestampPointer> {
        let mut packets = Vec::new();
        for (timestamp_ns, pointer) in &self.timestamp_index {
            if *timestamp_ns >= start_ns && *timestamp_ns <= end_ns {
                packets.push(pointer);
            }
        }
        packets.sort_by_key(|p| p.entry.timestamp_ns);
        packets
    }
}
