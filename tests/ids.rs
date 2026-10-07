use selmem::{Channel, EncodeInput, EntityProfile, SelectiveMemory};

#[test]
fn loading_an_older_book_cannot_overwrite_live_traces() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire")).detach_clock();
    let mut first = EncodeInput::new("The appointment is Tuesday in room A.");
    first.channel = Channel::World;
    let first_id = mem.live_with(first).trace_id.unwrap();

    // Loading another visitor's older/empty book must not rewind the shared IDs.
    selmem::persist::bump_id_counter(&selmem::MemoryStore::default());

    let mut second = EncodeInput::new("The appointment is Wednesday in room B.");
    second.channel = Channel::World;
    let second_id = mem.live_with(second).trace_id.unwrap();
    assert_ne!(first_id, second_id);
    assert_eq!(mem.store.traces.len(), 2);
    assert_eq!(mem.store.archives.len(), 2);
    assert!(mem.audit(&first_id).unwrap().contains("Tuesday"));
    assert!(mem.audit(&second_id).unwrap().contains("Wednesday"));
}
