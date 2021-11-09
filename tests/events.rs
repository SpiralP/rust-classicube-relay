use classicube_helpers::events::plugin_messages;
use classicube_relay::{events::*, packet::*};
use std::{cell::Cell, rc::Rc};

#[path = "logger.rs"]
mod logger;

#[test]
fn test_events_invalid_channel() {
    self::logger::initialize(true, false);

    assert!(RelayListener::new(20).is_err());
}

#[test]
fn test_events_invalid_continue() {
    self::logger::initialize(true, false);

    let called = Rc::new(Cell::new(None));
    let mut listener = RelayListener::new(200).unwrap();
    {
        let called = called.clone();
        listener.on(Box::new(move |a, b| {
            called.set(Some((a, b.to_vec())));
        }));
    }
    {
        let mut data_part = vec![];
        data_part.resize(ContinuePacket::DATA_PART_LENGTH, 0);
        let mut data = Packet::Continue(ContinuePacket::new(1, data_part).unwrap())
            .encode()
            .unwrap();
        plugin_messages::ReceivedEventHandler::raise(200, data.as_mut_ptr());
    }

    assert!(called.take().is_none());
}

#[test]
fn test_events_single_start_packet() {
    self::logger::initialize(true, false);

    let called = Rc::new(Cell::new(None));
    let mut listener = RelayListener::new(200).unwrap();
    {
        let called = called.clone();
        listener.on(Box::new(move |a, b| {
            called.set(Some((a, b.to_vec())));
        }));
    }

    let mut data_part = vec![];
    data_part.resize(StartPacket::DATA_PART_LENGTH, 0);
    let mut data = Packet::Start(StartPacket::new(1, PlayerScope::new(2), 2, data_part).unwrap())
        .encode()
        .unwrap();
    plugin_messages::ReceivedEventHandler::raise(200, data.as_mut_ptr());

    let args = called.take().unwrap();
    assert_eq!(args.0, 2);
    assert_eq!(args.1, vec![0; 2]);
}

#[test]
fn test_events_multiple_packets() {
    self::logger::initialize(true, false);

    let called = Rc::new(Cell::new(None));
    let mut listener = RelayListener::new(200).unwrap();
    {
        let called = called.clone();
        listener.on(Box::new(move |a, b| {
            called.set(Some((a, b.to_vec())));
        }));
    }

    {
        let mut data_part = vec![];
        data_part.resize(StartPacket::DATA_PART_LENGTH, 0);
        let mut data =
            Packet::Start(StartPacket::new(1, PlayerScope::new(2), 64, data_part).unwrap())
                .encode()
                .unwrap();
        plugin_messages::ReceivedEventHandler::raise(200, data.as_mut_ptr());
    }
    {
        let mut data_part = vec![];
        data_part.resize(ContinuePacket::DATA_PART_LENGTH, 0);
        let mut data = Packet::Continue(ContinuePacket::new(1, data_part).unwrap())
            .encode()
            .unwrap();
        plugin_messages::ReceivedEventHandler::raise(200, data.as_mut_ptr());
    }

    let args = called.take().unwrap();
    assert_eq!(args.0, 2);
    assert_eq!(args.1, vec![0; 64]);
}

#[test]
fn test_events_restart() {
    self::logger::initialize(true, true);

    let called = Rc::new(Cell::new(None));
    let mut listener = RelayListener::new(200).unwrap();
    {
        let called = called.clone();
        listener.on(Box::new(move |a, b| {
            called.set(Some((a, b.to_vec())));
        }));
    }

    {
        let mut data_part = vec![];
        data_part.resize(StartPacket::DATA_PART_LENGTH, 0);
        let mut data =
            Packet::Start(StartPacket::new(1, PlayerScope::new(2), 64, data_part).unwrap())
                .encode()
                .unwrap();
        plugin_messages::ReceivedEventHandler::raise(200, data.as_mut_ptr());
    }
    {
        let mut data_part = vec![];
        data_part.resize(StartPacket::DATA_PART_LENGTH, 0);
        let mut data =
            Packet::Start(StartPacket::new(1, PlayerScope::new(2), 10, data_part).unwrap())
                .encode()
                .unwrap();
        plugin_messages::ReceivedEventHandler::raise(200, data.as_mut_ptr());
    }

    let args = called.take().unwrap();
    assert_eq!(args.0, 2);
    assert_eq!(args.1, vec![0; 10]);
}
