use classicube_relay::packet::*;
use std::io::Cursor;

#[test]
fn test_decode_flags() {
    let data = &mut vec![
        // is_packet_start: true; stream_id: 3
        0b1000_0011,
    ];
    data.resize(PLUGIN_MESSAGE_DATA_LENGTH, 0);
    let flags = Flags::decode(&mut Cursor::new(data)).unwrap();
    assert_eq!(flags.is_packet_start, true);
    assert_eq!(flags.stream_id, 3);

    let data = &mut vec![
        // is_packet_start: true; stream_id: 3
        0b0000_0011,
    ];
    data.resize(PLUGIN_MESSAGE_DATA_LENGTH, 0);
    let flags = Flags::decode(&mut Cursor::new(data)).unwrap();
    assert_eq!(flags.is_packet_start, false);
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
    data.resize(PLUGIN_MESSAGE_DATA_LENGTH, 0);

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
    data.resize(PLUGIN_MESSAGE_DATA_LENGTH, 0);
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
    data.resize(PLUGIN_MESSAGE_DATA_LENGTH, 0);

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
fn test_make_packets() {
    // test 0 length
    let mut packets = Packet::make_packets(&vec![], Scope::Player { player_id: 0 }).unwrap();
    assert_eq!(packets.len(), 1);
    if let Packet::Start(StartPacket {
        stream_id,
        scope: _scope,
        data_length,
        data_part,
    }) = packets.remove(0)
    {
        assert_eq!(stream_id, 0);
        assert_eq!(data_length, 0);
        assert_eq!(data_part, vec![0; 59]);
    } else {
        unreachable!();
    }

    // test 1 single start packet
    let mut packets = Packet::make_packets(b"helloooo", Scope::Player { player_id: 0 }).unwrap();
    assert_eq!(packets.len(), 1);
    if let Packet::Start(StartPacket {
        stream_id,
        scope: _scope,
        data_length,
        data_part,
    }) = packets.remove(0)
    {
        assert_eq!(stream_id, 1);
        assert_eq!(data_length, 8);
        let mut v = b"helloooo".to_vec();
        v.resize(59, 0);
        assert_eq!(data_part, v);
    } else {
        unreachable!();
    }

    // test multiple packets
    let mut packets = Packet::make_packets(&vec![123; 64], Scope::Player { player_id: 0 }).unwrap();
    assert_eq!(packets.len(), 2);
    if let Packet::Start(StartPacket {
        stream_id,
        scope: _scope,
        data_length,
        data_part,
    }) = packets.remove(0)
    {
        assert_eq!(stream_id, 2);
        assert_eq!(data_length, 64);
        assert_eq!(data_part, vec![123; 59]);
    } else {
        unreachable!();
    }
    if let Packet::Continue(ContinuePacket {
        stream_id,
        data_part,
    }) = packets.remove(0)
    {
        assert_eq!(stream_id, 2);
        let mut v = vec![123; 5];
        v.resize(63, 0);
        assert_eq!(data_part, v);
    } else {
        unreachable!();
    }

    // test max size single packet
    let mut packets = Packet::make_packets(&vec![123; 59], Scope::Player { player_id: 0 }).unwrap();
    assert_eq!(packets.len(), 1);
    if let Packet::Start(StartPacket {
        stream_id,
        scope: _scope,
        data_length,
        data_part,
    }) = packets.remove(0)
    {
        assert_eq!(stream_id, 3);
        assert_eq!(data_length, 59);
        assert_eq!(data_part, vec![123; 59]);
    } else {
        unreachable!();
    }

    // test max size
    assert!(Packet::make_packets(
        &vec![123; u16::MAX as usize + 1],
        Scope::Player { player_id: 0 }
    )
    .is_err());

    let mut packets = Packet::make_packets(
        &vec![123; u16::MAX as usize],
        Scope::Player { player_id: 0 },
    )
    .unwrap();
    assert_eq!(
        packets.len(),
        (1.0 + ((65535.0 - 59.0) / 63.0 as f32).ceil()) as usize
    );
    if let Packet::Start(StartPacket {
        stream_id,
        scope: _scope,
        data_length,
        data_part,
    }) = packets.remove(0)
    {
        assert_eq!(stream_id, 4);
        assert_eq!(data_length, u16::MAX);
        assert_eq!(data_part, vec![123; 59]);
    } else {
        unreachable!();
    }
    while !packets.is_empty() {
        if let Packet::Continue(ContinuePacket {
            stream_id,
            data_part,
        }) = packets.remove(0)
        {
            assert_eq!(stream_id, 4);
            if !packets.is_empty() {
                assert_eq!(data_part, vec![123; 63]);
            } else {
                // last packet is smaller
                let mut v = vec![123; 19];
                v.resize(63, 0);
                assert_eq!(data_part, v);
            }
        } else {
            unreachable!();
        }
    }
    // 20 extra bytes
    // ((1 + (2^16 - 59)/63) - 1040) * 63

    // test "no free outgoing ids"
    for _ in 0..128 - 5 {
        Packet::make_packets(b"", Scope::Player { player_id: 0 }).unwrap();
    }
    assert!(Packet::make_packets(b"", Scope::Player { player_id: 0 }).is_err());
}
