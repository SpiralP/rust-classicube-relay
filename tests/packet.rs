use classicube_relay::packet::*;
use std::io::Cursor;

#[test]
fn test_decode_flags() {
    let data = &mut vec![
        // is_packet_start: true; stream_id: 3
        0b1000_0011,
    ];
    data.resize(Packet::DATA_LENGTH, 0);
    let flags = Flags::decode(&mut Cursor::new(data)).unwrap();
    assert!(flags.is_packet_start);
    assert_eq!(flags.stream_id, 3);

    let data = &mut vec![
        // is_packet_start: true; stream_id: 3
        0b0000_0011,
    ];
    data.resize(Packet::DATA_LENGTH, 0);
    let flags = Flags::decode(&mut Cursor::new(data)).unwrap();
    assert!(!flags.is_packet_start);
    assert_eq!(flags.stream_id, 3);
}

#[test]
fn test_decode_start_packet() {
    let mut data = vec![
        // is_packet_start: true; stream_id: 3
        0b1000_0011,
        // Player
        0x00,
        // player_id: 0
        0x00,
        // data_length: 1
        0x00,
        0x01,
        // data_part
        0xFF,
    ];
    data.resize(Packet::DATA_LENGTH, 0);

    let packet = Packet::decode(&mut Cursor::new(data)).unwrap();
    if let Packet::Start(StartPacket {
        stream_id,
        scope: _scope,
        data_length,
        data_part,
    }) = packet
    {
        assert_eq!(stream_id, 3);
        assert_eq!(data_length, 1);
        let mut v = vec![0xFF];
        v.resize(59, 0);
        assert_eq!(data_part, v);
    } else {
        unreachable!();
    }

    let mut data = vec![
        // is_packet_start; stream_id: 3
        0b1000_0011,
        // invalid
        0xFF,
    ];
    data.resize(Packet::DATA_LENGTH, 0);
    assert!(Packet::decode(&mut Cursor::new(data)).is_err());
}

#[test]
fn test_decode_continue_packet() {
    let mut data = vec![
        // is_packet_start: false; stream_id: 3
        0b0000_0011,
        // data_part
        0xFF,
    ];
    data.resize(Packet::DATA_LENGTH, 0);

    let packet = Packet::decode(&mut Cursor::new(data)).unwrap();
    if let Packet::Continue(ContinuePacket {
        stream_id,
        data_part,
    }) = packet
    {
        assert_eq!(stream_id, 3);
        let mut v = vec![0xFF];
        v.resize(63, 0);
        assert_eq!(data_part, v);
    } else {
        unreachable!();
    }
}

#[test]
fn test_encode_decode() {
    let mut data_stream = Cursor::new(vec![]);
    let packet = Packet::Start(
        StartPacket::new_reader(1, PlayerScope::new(2), 3, &mut data_stream).unwrap(),
    );
    let packet_data = packet.encode().unwrap();
    let decoded_packet = Packet::decode(&mut Cursor::new(packet_data)).unwrap();

    assert_eq!(packet, decoded_packet);
}
