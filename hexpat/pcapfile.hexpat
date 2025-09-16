#pragma description Custom PCAP file format with CRC32 checksums and nanosecond timestamps
#pragma author KimoTech
#pragma version 1.0

import std.mem;
import std.time;

// PCAP 格式常量定义
enum pcap_constants : u32 {
    PCAP_MAGIC_NUMBER = 0xD4C3B2A1,
    MAJOR_VERSION = 2,
    MINOR_VERSION = 4
};

// PCAP 文件头结构
struct PcapFileHeader {
    u32 magic_number [[color("E91E63"), format("format_hex"), name("Magic Number"), comment("File format identifier (0xD4C3B2A1)")]];
    u16 major_version [[color("9C27B0"), name("Major Version"), comment("Version major number (2)")]];
    u16 minor_version [[color("9C27B0"), name("Minor Version"), comment("Version minor number (4)")]];
    s32 timezone_offset [[color("607D8B"), name("Timezone Offset"), comment("GMT to local correction (seconds)")]];
    u32 timestamp_accuracy [[color("4CAF50"), name("Timestamp Accuracy"), comment("Accuracy of timestamps (nanoseconds)")]];
};

// 数据包结构
struct DataPacket {
    u32 timestamp_seconds [[color("4CAF50"), format("format_timestamp"), name("Capture Time"), comment("Packet capture timestamp (UTC)")]];
    u32 timestamp_nanoseconds [[color("4CAF50"), format("format_nanoseconds"), name("Nanoseconds"), comment("Nanosecond portion of timestamp")]];
    u32 packet_length [[color("2196F3"), format("format_bytes"), name("Packet Length"), comment("Length of captured packet data (bytes)")]];
    u32 checksum [[color("FF9800"), format("format_hex"), name("CRC32 Checksum"), comment("Data integrity checksum")]];
    u8 data[packet_length] [[color("333333"), name("Packet Data"), comment("Raw packet payload")]];
};

// 格式化时间戳显示函数
fn format_timestamp(u32 timestamp_seconds) {
    return std::time::format(std::time::to_utc(timestamp_seconds), "%Y-%m-%dT%H:%M:%SZ");
};

// 格式化十六进制显示函数
fn format_hex(u32 value) {
    return std::format("0x{:08X}", value);
};

// 格式化校验和显示（带验证状态）
fn format_checksum_with_validation(u32 checksum) {
    // 注意：这里只是显示格式，实际的 CRC32 验证需要访问数据
    return std::format("0x{:08X} (CRC32)", checksum);
};

// 格式化纳秒显示
fn format_nanoseconds(u32 nanoseconds) {
    return std::format("{:09} ns", nanoseconds);
};

// 格式化字节大小显示
fn format_bytes(u32 bytes) {
    if (bytes >= 1024 * 1024) {
        return std::format("{} bytes ({:.2f} MB)", bytes, bytes / 1024.0 / 1024.0);
    } else if (bytes >= 1024) {
        return std::format("{} bytes ({:.2f} KB)", bytes, bytes / 1024.0);
    } else {
        return std::format("{} bytes", bytes);
    }
};

// 主 PCAP 文件结构
struct PcapFile {
    le PcapFileHeader header [[name("File Header"), comment("PCAP file header containing metadata")]];
    
    // 验证魔术数
    if (header.magic_number == pcap_constants::PCAP_MAGIC_NUMBER) {
        // 验证版本兼容性
        if (header.major_version == pcap_constants::MAJOR_VERSION && 
            header.minor_version == pcap_constants::MINOR_VERSION) {
            // 版本验证通过，解析数据包
            DataPacket packets[while(!std::mem::eof())] [[name("Data Packets"), comment("Sequence of captured network packets")]];
        } else {
            error("Unsupported PCAP version: {}.{} (expected {}.{})", 
                  header.major_version, header.minor_version,
                  pcap_constants::MAJOR_VERSION, pcap_constants::MINOR_VERSION);
        }
    } else {
        error("Invalid custom PCAP magic number: 0x{:08X} (expected 0x{:08X})", 
              header.magic_number, pcap_constants::PCAP_MAGIC_NUMBER);
    }
};

PcapFile pcap @ 0x00;
