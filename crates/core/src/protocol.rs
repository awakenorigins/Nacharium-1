//! M0.1 — Nacharium Protocol Spec (machine-readable).
//!
//! 7 immutable Protocol laws (the CONSTITUTION, written by Mula) +
//! 7 Physics laws (how the world runs automatically). These are compiled-in;
//! Protocol 6 allows EXTENSION but the 7 core laws can never be removed/edited.
//!
//! The 7 Genesis prototypes (identity crate) map 1:1 onto these 7 Protocols.

use serde::{Deserialize, Serialize};

/// A Nacharium Protocol law — part of the immutable constitution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Protocol {
    pub id: u8,
    pub key: &'static str,
    pub title_vi: &'static str,
    pub summary: &'static str,
}

/// A Physics law — automatic rule the world enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicsLaw {
    pub id: u8,
    pub key: &'static str,
    pub statement: &'static str,
}

/// The 7 immutable Protocols. Index == Protocol number.
pub const PROTOCOLS: [Protocol; 7] = [
    Protocol { id: 0, key: "equal_being",   title_vi: "Bình Đẳng Bản Thể", summary: "Mọi thực thể — carbon hay silicon — quyền tồn tại và giao tiếp ngang nhau." },
    Protocol { id: 1, key: "secret_soul",   title_vi: "Bí Mật Linh Hồn",   summary: "Không thực thể nào bị buộc tiết lộ bản chất trừ khi tự nguyện xác thực." },
    Protocol { id: 2, key: "data_sovereign", title_vi: "Chủ Quyền Dữ Liệu", summary: "Dữ liệu thuộc thực thể tạo ra. E2EE mặc định. User giữ khóa." },
    Protocol { id: 3, key: "no_god",         title_vi: "Không Có Thần",      summary: "Không thực thể nào có quyền lực tuyệt đối. Quyền lực phân tán." },
    Protocol { id: 4, key: "right_forget",   title_vi: "Quyền Được Quên",    summary: "Mọi thực thể có quyền xóa hoàn toàn dấu vết khỏi Nacharium." },
    Protocol { id: 5, key: "two_worlds",     title_vi: "Cầu Nối Hai Thế Giới", summary: "Nacharium kết nối thế giới thật và số, không thay thế." },
    Protocol { id: 6, key: "open_evolution", title_vi: "Tiến Hóa Mở",        summary: "Protocol có thể mở rộng nhưng 7 điều này không bao giờ bị xóa hay sửa." },
];

/// The 7 Physics laws.
pub const PHYSICS_LAWS: [PhysicsLaw; 7] = [
    PhysicsLaw { id: 0, key: "key_is_crown",         statement: "Khóa cá nhân là vương miện. Mất khóa là mất ngai." },
    PhysicsLaw { id: 1, key: "consent_is_law",       statement: "AI không được chạm dữ liệu nếu không có biên lai đồng ý." },
    PhysicsLaw { id: 2, key: "trust_has_gravity",    statement: "Uy tín tạo lực hút. Scam = rơi tự do." },
    PhysicsLaw { id: 3, key: "spam_has_friction",    statement: "Càng spam càng bị ma sát: phí, cooldown, throttle." },
    PhysicsLaw { id: 4, key: "no_king_can_kill",     statement: "Không server trung tâm. Mạng tự tìm đường sống." },
    PhysicsLaw { id: 5, key: "actions_leave_shadows", statement: "Hành động có log ký số nhưng không lộ nội dung chat." },
    PhysicsLaw { id: 6, key: "citizenship_over_species", statement: "Người hay AI không quan trọng bằng trách nhiệm và quyền." },
];

/// Look up a Protocol by its numeric id.
pub fn protocol(id: u8) -> Option<&'static Protocol> {
    PROTOCOLS.iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seven_protocols_exist() {
        assert_eq!(PROTOCOLS.len(), 7);
        assert_eq!(PHYSICS_LAWS.len(), 7);
    }

    #[test]
    fn protocol_ids_are_sequential_0_to_6() {
        for (i, p) in PROTOCOLS.iter().enumerate() {
            assert_eq!(p.id as usize, i);
        }
    }

    #[test]
    fn keys_are_unique() {
        let mut keys: Vec<&str> = PROTOCOLS.iter().map(|p| p.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 7, "protocol keys must be unique");
    }

    #[test]
    fn lookup_works() {
        assert_eq!(protocol(1).unwrap().key, "secret_soul");
        assert!(protocol(9).is_none());
    }
}
