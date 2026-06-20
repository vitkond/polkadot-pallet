use crate::{mock::*, Error, NextNoteId, NoteCount, Notes as NotesStorage};
use frame_support::{assert_noop, assert_ok};
use codec::Encode;
use frame_support::traits::OnRuntimeUpgrade;

#[test]
fn create_note_works() {
    new_test_ext().execute_with(|| {
        let owner = 1;
        let note_content = b"Hello, Substrate!".to_vec();

        assert_ok!(Notes::create_note(
            RuntimeOrigin::signed(owner),
            note_content.clone()
        ));

        let next_note_id = NextNoteId::<Test>::get(&owner);
        assert_eq!(next_note_id, 1);

        let stored_note = NotesStorage::<Test>::get(&owner, 0).unwrap();
        assert_eq!(stored_note.content, note_content);
        assert_eq!(stored_note.created_at, Some(0));
    });
}

#[test]
fn create_note_too_long_fails() {
    new_test_ext().execute_with(|| {
        let owner = 1;
        let long_content = vec![0u8; 513];

        assert_noop!(
            Notes::create_note(RuntimeOrigin::signed(owner), long_content),
            Error::<Test>::NoteTooLong
        );
    });
}

#[test]
fn delete_note_works() {
    new_test_ext().execute_with(|| {
        let owner = 1;
        let note_content = b"Hello, Substrate!".to_vec();

        assert_ok!(Notes::create_note(
            RuntimeOrigin::signed(owner),
            note_content
        ));
        assert_eq!(NoteCount::<Test>::get(&owner), 1);

        assert_ok!(Notes::delete_note(RuntimeOrigin::signed(owner), 0));

        assert_eq!(NoteCount::<Test>::get(&owner), 0);
        assert!(NotesStorage::<Test>::get(&owner, 0).is_none());
    });
}

#[test]
fn delete_missing_note_does_not_decrease_count() {
    new_test_ext().execute_with(|| {
        let owner = 1;

        assert_noop!(
            Notes::delete_note(RuntimeOrigin::signed(owner), 0),
            Error::<Test>::NoteNotFound
        );

        assert_eq!(NoteCount::<Test>::get(&owner), 0);
    });
}

#[test]
fn create_note_sets_created_at() {
    new_test_ext().execute_with(|| {
        System::set_block_number(10);

        assert_ok!(Notes::create_note(
            RuntimeOrigin::signed(1),
            b"hello".to_vec()
        ));

        let note = NotesStorage::<Test>::get(1, 0).unwrap();

        assert_eq!(note.content, b"hello".to_vec());
        assert_eq!(note.created_at, Some(10));
    });
}

#[test]
fn migration_v0_to_v1_works() {
    new_test_ext().execute_with(|| {
        let owner = 1;
        let note_id = 0;
        let old_content = b"old note".to_vec();

        let key = NotesStorage::<Test>::hashed_key_for(owner, note_id);

        sp_io::storage::set(&key, &old_content.encode());

        Notes::on_runtime_upgrade();

        let migrated_note = NotesStorage::<Test>::get(owner, note_id).unwrap();

        assert_eq!(migrated_note.content, old_content);
        assert_eq!(migrated_note.created_at, None);
    });
}