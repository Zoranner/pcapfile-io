#pragma description PCAP file

import std.mem;

// PCAP 格式常量定义
enum pcap_constants : u32 {
    PCAP_MAGIC_NUMBER = 0xD4C3B2A1,
    MAJOR_VERSION = 2,
    MINOR_VERSION = 4
};

// PCAP 文件头结构
struct PcapFileHeader {
    u32 magic_number [[color("E91E63")]];           // 魔术数，固定值 0xD4C3B2A1 - 粉红色
    u16 major_version [[color("9C27B0")]];         // 主版本号，固定值 0x0002 - 紫色
    u16 minor_version [[color("9C27B0")]];         // 次版本号，固定值 0x0004 - 紫色
    s32 timezone_offset [[color("607D8B")]];       // 时区偏移量（秒） - 蓝灰色
    u32 timestamp_accuracy [[color("4CAF50")]];    // 时间戳精度（纳秒） - 绿色
};

// 数据包结构
struct DataPacket {
    u32 timestamp_seconds [[color("4CAF50")]];      // 时间戳秒部分（UTC） - 绿色
    u32 timestamp_nanoseconds [[color("4CAF50")]]; // 时间戳纳秒部分（UTC） - 绿色
    u32 packet_length [[color("2196F3")]];         // 数据包长度（字节） - 蓝色
    u32 checksum [[color("FF9800")]];              // 数据包校验和（CRC32） - 橙色
    u8 data[packet_length] [[color("333333")]];    // 数据包内容 - 灰色
};

// 主 PCAP 文件结构
struct PcapFile {
    le PcapFileHeader header;
    if (header.magic_number == pcap_constants::PCAP_MAGIC_NUMBER) {
        // 魔术数验证通过，解析数据包
        DataPacket packets[while(!std::mem::eof())];
    } else {
        error("Invalid custom PCAP magic number: 0x{:08X}", header.magic_number);
    }
};

PcapFile pcap @ 0x00;
