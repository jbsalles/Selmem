use selmem::{EntityProfile, SelectiveMemory};
use std::collections::HashMap;
use std::io::{self, Read};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type Organ = Arc<Mutex<Option<SelectiveMemory>>>;
const TTL: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_SESSIONS: usize = 64;

struct Session {
    organ: Organ,
    created: Instant,
}

pub struct Sessions {
    entries: Mutex<HashMap<String, Session>>,
    profile: EntityProfile,
}

impl Sessions {
    pub fn new(profile: EntityProfile) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            profile,
        }
    }

    pub fn resolve(&self, id: &str, create: bool) -> io::Result<Option<(String, Organ)>> {
        let now = Instant::now();
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| io::Error::other("sessions locked"))?;
        entries.retain(|_, s| now.duration_since(s.created) < TTL);
        if let Some(session) = entries.get(id) {
            return Ok(Some((id.into(), Arc::clone(&session.organ))));
        }
        if !create || entries.len() >= MAX_SESSIONS {
            return Ok(None);
        }
        let mut bytes = [0u8; 32];
        std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
        let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        // No path or configured narrator: keys, memory and conversation stay in RAM.
        let organ = Arc::new(Mutex::new(Some(
            SelectiveMemory::new(self.profile.clone()).detach_clock(),
        )));
        entries.insert(
            id.clone(),
            Session {
                organ: Arc::clone(&organ),
                created: now,
            },
        );
        Ok(Some((id, organ)))
    }

    /// Drop expired memory and keys even when no visitor sends another request.
    pub fn purge_expired(&self) -> io::Result<()> {
        let now = Instant::now();
        self.entries.lock().map_err(|_| io::Error::other("sessions locked"))?
            .retain(|_, s| now.duration_since(s.created) < TTL);
        Ok(())
    }
}

pub fn cookie_id(header: &str) -> &str {
    header
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(name, value)| (name == "selmem_session").then_some(value))
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visitors_get_independent_memories_and_keys() {
        let sessions = Sessions::new(EntityProfile::tender("Demo"));
        let (a, organ_a) = sessions.resolve("", true).unwrap().unwrap();
        let (b, organ_b) = sessions.resolve("", true).unwrap().unwrap();
        assert_ne!(a, b);
        assert!(!Arc::ptr_eq(&organ_a, &organ_b));
        organ_a.lock().unwrap().as_mut().unwrap().profile.name = "A".into();
        assert_eq!(
            organ_b.lock().unwrap().as_ref().unwrap().profile.name,
            "Demo"
        );
        assert!(organ_b.lock().unwrap().as_ref().unwrap().llm.key.is_none());
        assert!(Arc::ptr_eq(
            &organ_a,
            &sessions.resolve(&a, false).unwrap().unwrap().1
        ));
        assert!(sessions.resolve("forged", false).unwrap().is_none());
    }

    #[test]
    fn daily_sessions_expire_even_when_active_and_capacity_is_bounded() {
        let sessions = Sessions::new(EntityProfile::tender("Demo"));
        let (id, _) = sessions.resolve("", true).unwrap().unwrap();
        sessions
            .entries
            .lock()
            .unwrap()
            .get_mut(&id)
            .unwrap()
            .created = Instant::now() - TTL;
        assert!(sessions.resolve(&id, false).unwrap().is_none());
        for _ in 0..MAX_SESSIONS {
            assert!(sessions.resolve("", true).unwrap().is_some());
        }
        assert!(sessions.resolve("", true).unwrap().is_none());
    }

    #[test]
    fn returning_visitors_do_not_extend_daily_lifetime() {
        let sessions = Sessions::new(EntityProfile::tender("Claire"));
        let (id, _) = sessions.resolve("", true).unwrap().unwrap();
        let created = Instant::now() - Duration::from_secs(23 * 60 * 60);
        sessions.entries.lock().unwrap().get_mut(&id).unwrap().created = created;
        assert!(sessions.resolve(&id, false).unwrap().is_some());
        assert_eq!(sessions.entries.lock().unwrap().get(&id).unwrap().created, created);
        sessions.entries.lock().unwrap().get_mut(&id).unwrap().created = Instant::now() - TTL;
        sessions.purge_expired().unwrap();
        assert!(sessions.entries.lock().unwrap().is_empty());
    }
}
