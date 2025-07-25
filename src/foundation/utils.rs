use chrono::{DateTime, Timelike, Utc};

/// 字节数组扩展方法
pub trait ByteArrayExtensions {
    /// 获取字节数组的子数组
    fn sub_array(&self, start_index: usize, length: usize) -> Result<Vec<u8>, String>;

    /// 将字节数组转换为十六进制字符串
    fn to_hex_string(&self, separator: &str) -> String;

    /// 将字节数组转换为Base64字符串
    fn to_base64_string(&self) -> String;

    /// 将字节数组转换为UTF8字符串
    fn to_utf8_string(&self) -> Result<String, String>;

    /// 比较两个字节数组是否相等
    fn equals(&self, other: &[u8]) -> bool;

    /// 计算字节数组的哈希值
    fn get_hash_code(&self) -> u32;
}

impl ByteArrayExtensions for [u8] {
    fn sub_array(&self, start_index: usize, length: usize) -> Result<Vec<u8>, String> {
        if start_index >= self.len() {
            return Err("起始索引超出范围".to_string());
        }

        if start_index + length > self.len() {
            return Err("长度超出范围".to_string());
        }

        Ok(self[start_index..start_index + length].to_vec())
    }

    fn to_hex_string(&self, separator: &str) -> String {
        if self.is_empty() {
            return String::new();
        }

        let mut result = String::with_capacity(self.len() * (2 + separator.len()));
        for (i, &byte) in self.iter().enumerate() {
            if i > 0 && !separator.is_empty() {
                result.push_str(separator);
            }
            result.push_str(&format!("{:02X}", byte));
        }
        result
    }

    fn to_base64_string(&self) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, self)
    }

    fn to_utf8_string(&self) -> Result<String, String> {
        String::from_utf8(self.to_vec()).map_err(|e| format!("UTF8解码失败: {}", e))
    }

    fn equals(&self, other: &[u8]) -> bool {
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }

    fn get_hash_code(&self) -> u32 {
        let mut hash = 17u32;
        for &byte in self {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }
        hash
    }
}

/// DateTime扩展方法
pub trait DateTimeExtensions {
    /// 将 DateTime 转换为 Unix 时间戳(毫秒)
    fn to_unix_time_milliseconds(&self) -> i64;

    /// 将 Unix 时间戳(毫秒)转换为 DateTime
    fn from_unix_time_milliseconds(milliseconds: i64) -> DateTime<Utc>;

    /// 将 DateTime 转换为 Unix 时间戳(秒)
    fn to_unix_time_seconds(&self) -> u32;

    /// 获取 DateTime 的纳秒部分
    fn get_nanoseconds(&self) -> u32;

    /// 将 Unix 时间戳(秒)和纳秒部分转换为 DateTime
    fn from_unix_time_with_nanoseconds(seconds: u32, nanoseconds: u32) -> DateTime<Utc>;
}

impl DateTimeExtensions for DateTime<Utc> {
    fn to_unix_time_milliseconds(&self) -> i64 {
        self.timestamp_millis()
    }

    fn from_unix_time_milliseconds(milliseconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(milliseconds).unwrap_or_default()
    }

    fn to_unix_time_seconds(&self) -> u32 {
        self.timestamp() as u32
    }

    fn get_nanoseconds(&self) -> u32 {
        self.nanosecond()
    }

    fn from_unix_time_with_nanoseconds(seconds: u32, nanoseconds: u32) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds as i64, nanoseconds).unwrap_or_default()
    }
}

/// 计算CRC32校验和
pub fn calculate_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;

    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }

    !crc
}

/// 二进制转换工具
pub mod binary_converter {
    /// 从字节数组读取小端序整数
    pub fn read_le_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
        if offset + 4 > bytes.len() {
            return Err("字节数组长度不足".to_string());
        }

        Ok(u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]))
    }

    /// 从字节数组读取小端序16位整数
    pub fn read_le_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
        if offset + 2 > bytes.len() {
            return Err("字节数组长度不足".to_string());
        }

        Ok(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
    }

    /// 将整数写入字节数组（小端序）
    pub fn write_le_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), String> {
        if offset + 4 > bytes.len() {
            return Err("字节数组长度不足".to_string());
        }

        let value_bytes = value.to_le_bytes();
        bytes[offset..offset + 4].copy_from_slice(&value_bytes);
        Ok(())
    }

    /// 将16位整数写入字节数组（小端序）
    pub fn write_le_u16(bytes: &mut [u8], offset: usize, value: u16) -> Result<(), String> {
        if offset + 2 > bytes.len() {
            return Err("字节数组长度不足".to_string());
        }

        let value_bytes = value.to_le_bytes();
        bytes[offset..offset + 2].copy_from_slice(&value_bytes);
        Ok(())
    }

    /// 将字符串转换为UTF8字节数组
    pub fn string_to_utf8_bytes(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    /// 从UTF8字节数组转换为字符串
    pub fn utf8_bytes_to_string(bytes: &[u8]) -> Result<String, String> {
        String::from_utf8(bytes.to_vec()).map_err(|e| format!("UTF8解码失败: {}", e))
    }

    /// 将字节数组转换为Base64字符串
    pub fn bytes_to_base64(bytes: &[u8]) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
    }

    /// 从Base64字符串转换为字节数组
    pub fn base64_to_bytes(base64_str: &str) -> Result<Vec<u8>, String> {
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_str)
            .map_err(|e| format!("Base64解码失败: {}", e))
    }
}


