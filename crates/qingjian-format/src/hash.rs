//! 能写进文件的字符串哈希索引：开放寻址表，槽里放条目编号，键本身留在调用方的 arena 里。
//!
//! 哈希函数必须跨版本、跨机器稳定（写文件的进程和读文件的进程算出来要一样），所以不用标准库或 foldhash
//! 带随机种子的哈希器，用固定的 FNV-1a 64。词级查表每键几千次，一次是「哈希 + 一两次探测 + 一次字符串比较」。

/// 空槽。
pub const EMPTY: u32 = u32::MAX;

/// FNV-1a 64 位，再过一遍 MurmurHash3 的 fmix64 终混：FNV 的低位对短的、字节模式相近的输入（UTF-8 中文全是
/// 0xE4–0xE9 开头的三字节）分布不好，而开放寻址表只用低位选槽，不混一下探测链会很长。
pub fn stable_hash(key: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in key.as_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    hash ^ (hash >> 33)
}

/// 装 `count` 条要多大的表：2 的幂、至少两倍条数（装载率 ≤ 0.5，线性探测平均一两步）。
pub fn capacity_for(count: usize) -> usize {
    (count.saturating_mul(2)).next_power_of_two().max(16)
}

/// 建表：条目编号 `0..count`，`key_of(id)` 给每条的键。键重复时后来的探测到已有的键会各占一槽，
/// 查找返回先放进去的那条；调用方应保证键唯一。
pub fn build<'a>(count: usize, key_of: impl Fn(u32) -> &'a str) -> Vec<u32> {
    let capacity = capacity_for(count);
    let mask = capacity - 1;
    let mut table = vec![EMPTY; capacity];
    for id in 0..count {
        let id = u32::try_from(id).expect("hash index holds at most u32::MAX entries");
        let mut slot = (stable_hash(key_of(id)) as usize) & mask;
        while table[slot] != EMPTY {
            slot = (slot + 1) & mask;
        }
        table[slot] = id;
    }
    table
}

/// 查找：表必须是 [`build`] 出来的（容量 2 的幂、至少一个空槽）。
pub fn find<'a>(table: &[u32], key: &str, key_of: impl Fn(u32) -> &'a str) -> Option<u32> {
    if table.is_empty() {
        return None;
    }
    let mask = table.len() - 1;
    let mut slot = (stable_hash(key) as usize) & mask;
    loop {
        let id = table[slot];
        if id == EMPTY {
            return None;
        }
        if key_of(id) == key {
            return Some(id);
        }
        slot = (slot + 1) & mask;
    }
}

/// 表是否合法：容量 2 的幂，且至少有一个空槽（否则 [`find`] 找不存在的键会死循环）。
pub fn is_valid(table: &[u32], count: usize) -> bool {
    !table.is_empty()
        && table.len().is_power_of_two()
        && table.iter().filter(|&&slot| slot != EMPTY).count() == count
        && table
            .iter()
            .all(|&slot| slot == EMPTY || (slot as usize) < count)
        && table.len() > count
}

#[cfg(test)]
mod tests {
    use super::*;

    const PINNED_EMPTY: u64 = 0xefd0_1f60_ba99_2926;
    const PINNED_KAIFA: u64 = 0x3758_0f72_eb7d_16d3;

    #[test]
    fn finds_every_key_and_rejects_unknown_ones() {
        let keys = ["我", "想", "去", "吃饭", "<s>", "开发", "开放"];
        let table = build(keys.len(), |id| keys[id as usize]);
        assert!(is_valid(&table, keys.len()));
        for (id, key) in keys.iter().enumerate() {
            assert_eq!(find(&table, key, |id| keys[id as usize]), Some(id as u32));
        }
        assert_eq!(find(&table, "没有", |id| keys[id as usize]), None);
        assert_eq!(find(&[], "我", |id| keys[id as usize]), None);
    }

    #[test]
    fn hash_is_stable() {
        // 固定值：换哈希函数会让已有的 .qj 文件全部失效，测试钉住它
        assert_eq!(stable_hash(""), PINNED_EMPTY);
        assert_eq!(stable_hash("开发"), PINNED_KAIFA);
    }
}
