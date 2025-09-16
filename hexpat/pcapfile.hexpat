#pragma description KimoTech PCAP file format
#pragma author KimoTech
#pragma version 1.1

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
    u32 magic_number [[color("E91E83"), format("format_hex"), name("Magic Number"), comment("File format identifier (0xD4C3B2A1)")]];
    u16 major_version [[color("9C27B0"), name("Major Version"), comment("Version major number (2)")]];
    u16 minor_version [[color("9C27B0"), name("Minor Version"), comment("Version minor number (4)")]];
    s32 timezone_offset [[color("EC6F50"), name("Timezone Offset"), comment("GMT to local correction (seconds)")]];
    u32 timestamp_accuracy [[color("705DFB"), name("Timestamp Accuracy"), comment("Accuracy of timestamps (nanoseconds)")]];
};

// 数据包结构（带容错处理）
struct DataPacket {
    u32 timestamp_seconds [[color("4CFF50"), format("format_timestamp"), name("Capture Time"), comment("Packet capture timestamp (UTC)")]];
    u32 timestamp_nanoseconds [[color("4CAF50"), format("format_nanoseconds"), name("Nanoseconds"), comment("Nanosecond portion of timestamp")]];
    u32 packet_length [[color("2196F3"), format("format_bytes"), name("Packet Length"), comment("Length of captured packet data (bytes)")]];
    u32 checksum [[color("FF98FF"), format("format_hex"), name("CRC32 Checksum"), comment("Data integrity checksum")]];
    
    // 容错的数据读取
    if (packet_length <= (std::mem::size() - $)) {
        // 正常的数据包（包括空数据包）
        if (packet_length > 0) {
            u8 data[packet_length] [[color("333333"), name("Packet Data"), comment("Raw packet payload")]];
        }
    } else {
        // 数据包长度超出文件边界，读取剩余数据
        u32 remaining_bytes = std::mem::size() - $;
        if (remaining_bytes > 0) {
            u8 partial_data[remaining_bytes] [[color("FFFF00"), name("Partial Data"), comment("Warning: Packet truncated at file boundary")]];
        } else {
            u8 boundary_error [[color("FF0000"), name("Boundary Error"), comment("Error: Packet extends beyond file boundary")]];
        }
    }
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

// 主 PCAP 文件结构（带容错处理）
struct PcapFile {
    le PcapFileHeader header [[name("File Header"), comment("PCAP file header containing metadata")]];
    
    // 魔术数验证
    if (header.magic_number == pcap_constants::PCAP_MAGIC_NUMBER) {
        // 版本兼容性检查
        if (header.major_version == pcap_constants::MAJOR_VERSION && 
            header.minor_version == pcap_constants::MINOR_VERSION) {
            // 版本验证通过，正常解析数据包
            DataPacket packets[while(!std::mem::eof())] [[name("Data Packets"), comment("Sequence of captured network packets")]];
        } else {
            // 版本不匹配，将所有剩余数据标记为错误
            u8 version_error_data[while(!std::mem::eof())] [[color("FF0000"), name("Version Error"), 
                comment("Error: Version mismatch - all data marked as invalid")]];
        }
    } else {
        // 魔术数不匹配，将所有剩余数据标记为错误
        u8 magic_error_data[while(!std::mem::eof())] [[color("FF0000"), name("Magic Number Error"), 
            comment("Error: Invalid magic number - all data marked as invalid")]];
    }
};

PcapFile PcapFile @ 0x00;
