#pragma description KimoTech PCAP file format
#pragma author KimoTech
#pragma version 1.2
#pragma pattern_limit 4294967295
#pragma array_limit 4294967295

import std.mem;
import std.time;

// PCAP 格式常量定义
enum pcap_constants : u32 {
    PCAP_MAGIC_NUMBER = 0xD4C3B2A1,
    MAJOR_VERSION = 2,
    MINOR_VERSION = 4
};

// PCAP 文件头结构
struct FileHeader {
    u32 magic_number [[color("FF00FF"), format("format_hex"), name("Magic Number"), comment("File format identifier (0xD4C3B2A1)")]];
    u16 major_version [[color("AF00AF"), name("Major Version"), comment("Version major number (2)")]];
    u16 minor_version [[color("9C27B0"), name("Minor Version"), comment("Version minor number (4)")]];
    s32 timezone_offset [[color("0076FF"), name("Time Offset"), comment("GMT to local correction (seconds)")]];
    u32 timestamp_accuracy [[color("705DFB"), name("Time Accuracy"), comment("Accuracy of timestamps (nanoseconds)")]];
};

// 数据包结构
struct DataPacket {
    u32 timestamp_seconds [[color("00FF00"), format("format_timestamp"), name("Capture Time"), comment("Packet capture timestamp (UTC)")]];
    u32 timestamp_nanoseconds [[color("00AF00"), format("format_nanoseconds"), name("Nanoseconds"), comment("Nanosecond portion of timestamp")]];
    u32 packet_length [[color("FFC800"), format("format_bytes"), name("Packet Length"), comment("Length of captured packet data (bytes)")]];
    u32 checksum [[color("FF98CF"), format("format_hex"), name("CRC32 Checksum"), comment("Data integrity checksum")]];
    
    // 检查数据包长度是否超出文件边界
    if (($ + packet_length) <= std::mem::size()) {
        // 正常跳过包体数据
        $ = $ + packet_length;
    } else {
        // 数据包长度超出文件边界，标记长度字段为错误并跳到文件末尾
        u32 error_length @ addressof(packet_length) [[color("FF0000"), format("format_bytes"), name("Invalid Packet Length"), 
            comment("Error: Packet length exceeds file boundary")]];
        $ = std::mem::size();
    }
    // 如果 packet_length == 0，不需要跳过任何数据
};

// 主 PCAP 文件结构（带容错处理）
struct PcapFile {
    le FileHeader header [[name("File Header"), comment("PCAP file header containing metadata")]];
    
    // 魔术数验证
    if (header.magic_number == pcap_constants::PCAP_MAGIC_NUMBER) {
        // 版本兼容性检查
        if (header.major_version == pcap_constants::MAJOR_VERSION && 
            header.minor_version == pcap_constants::MINOR_VERSION) {
            // 版本验证通过，正常解析数据包
            DataPacket packets[while(!std::mem::eof())] [[name("Data Packets"), comment("Sequence of captured network packets")]];
        } else {
            // 版本不匹配，标记版本错误并停止解析
            u8 error_version[4] @ addressof(header.major_version) [[color("FF0000"), name("Invalid Version"), 
                comment("Error: Expected version 2.4, got incompatible version")]];
        }
    } else {
        // 魔术数不匹配，标记魔术数错误并停止解析
        u32 error_magic @ addressof(header.magic_number) [[color("FF0000"), format("format_hex"), name("Invalid Magic Number"), 
            comment("Error: Expected 0xD4C3B2A1, this is not a valid PCAP file")]];
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

PcapFile PcapFile @ 0x00;
