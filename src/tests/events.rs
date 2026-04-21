use std::{cell::Cell, rc::Rc};

use classicube_helpers::events::net::PluginMessageReceivedEventHandler;

use crate::{events::*, packet::*};

// #[cfg(windows)]
#[test]
fn test_events_invalid_channel() {
    crate::tests::logger::initialize(true, false);

    assert!(RelayListener::new(20).is_err());
}

// #[cfg(windows)]
#[test]
fn test_events_invalid_continue() {
    crate::tests::logger::initialize(true, false);

    let called = Rc::new(Cell::new(None));
    let mut listener = RelayListener::new(200).unwrap();
    {
        let called = called.clone();
        listener.on(move |a, b| {
            called.set(Some((a, b.to_vec())));
        });
    }
    {
        let data_part = vec![0; ContinuePacket::DATA_PART_LENGTH];
        let mut data = Packet::Continue(ContinuePacket::new(1, data_part).unwrap())
            .encode()
            .unwrap();
        PluginMessageReceivedEventHandler::raise(200, data.as_mut_ptr());
    }

    assert!(called.take().is_none());
}

// #[cfg(windows)]
#[test]
fn test_events_single_start_packet() {
    crate::tests::logger::initialize(true, false);

    let called = Rc::new(Cell::new(None));
    let mut listener = RelayListener::new(200).unwrap();
    {
        let called = called.clone();
        listener.on(move |a, b| {
            called.set(Some((a, b.to_vec())));
        });
    }

    let data_part = vec![0; StartPacket::DATA_PART_LENGTH];
    let mut data = Packet::Start(StartPacket::new(1, PlayerScope::new(2), 2, data_part).unwrap())
        .encode()
        .unwrap();
    PluginMessageReceivedEventHandler::raise(200, data.as_mut_ptr());

    let args = called.take().unwrap();
    assert_eq!(args.0, 2);
    assert_eq!(args.1, vec![0; 2]);
}

// #[cfg(windows)]
#[test]
fn test_events_multiple_packets() {
    crate::tests::logger::initialize(true, false);

    let called = Rc::new(Cell::new(None));
    let mut listener = RelayListener::new(200).unwrap();
    {
        let called = called.clone();
        listener.on(move |a, b| {
            called.set(Some((a, b.to_vec())));
        });
    }

    {
        let data_part = vec![0; StartPacket::DATA_PART_LENGTH];
        let mut data =
            Packet::Start(StartPacket::new(1, PlayerScope::new(2), 64, data_part).unwrap())
                .encode()
                .unwrap();
        PluginMessageReceivedEventHandler::raise(200, data.as_mut_ptr());
    }
    {
        let data_part = vec![0; ContinuePacket::DATA_PART_LENGTH];
        let mut data = Packet::Continue(ContinuePacket::new(1, data_part).unwrap())
            .encode()
            .unwrap();
        PluginMessageReceivedEventHandler::raise(200, data.as_mut_ptr());
    }

    let args = called.take().unwrap();
    assert_eq!(args.0, 2);
    assert_eq!(args.1, vec![0; 64]);
}

// #[cfg(windows)]
#[test]
fn test_events_restart() {
    crate::tests::logger::initialize(true, true);

    let called = Rc::new(Cell::new(None));
    let mut listener = RelayListener::new(200).unwrap();
    {
        let called = called.clone();
        listener.on(move |a, b| {
            called.set(Some((a, b.to_vec())));
        });
    }

    {
        let data_part = vec![0; StartPacket::DATA_PART_LENGTH];
        let mut data =
            Packet::Start(StartPacket::new(1, PlayerScope::new(2), 64, data_part).unwrap())
                .encode()
                .unwrap();
        PluginMessageReceivedEventHandler::raise(200, data.as_mut_ptr());
    }
    {
        let data_part = vec![0; StartPacket::DATA_PART_LENGTH];
        let mut data =
            Packet::Start(StartPacket::new(1, PlayerScope::new(2), 10, data_part).unwrap())
                .encode()
                .unwrap();
        PluginMessageReceivedEventHandler::raise(200, data.as_mut_ptr());
    }

    let args = called.take().unwrap();
    assert_eq!(args.0, 2);
    assert_eq!(args.1, vec![0; 10]);
}
