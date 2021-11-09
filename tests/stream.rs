use classicube_relay::{packet::*, stream::*};

#[test]
fn test_stream() {
    // test 0 length
    let stream = Stream::new(vec![], PlayerScope::new(0)).unwrap();
    let mut packets = stream.packets().unwrap();
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
    let stream = Stream::new(b"helloooo".to_vec(), PlayerScope::new(0)).unwrap();
    let mut packets = stream.packets().unwrap();
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
    let stream = Stream::new(vec![123; 64], PlayerScope::new(0)).unwrap();
    let mut packets = stream.packets().unwrap();
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
    let stream = Stream::new(vec![123; 59], PlayerScope::new(0)).unwrap();
    let mut packets = stream.packets().unwrap();
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
    assert!(Stream::new(vec![123; u16::MAX as usize + 1], PlayerScope::new(0)).is_err());

    let stream = Stream::new(vec![123; u16::MAX as usize], PlayerScope::new(0)).unwrap();
    let mut packets = stream.packets().unwrap();
    assert_eq!(
        packets.len(),
        (1.0 + ((65535.0 - 59.0) / 63.0_f32).ceil()) as usize
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
}

#[test]
fn test_stream_frees_packet_id() {
    let mut streams = vec![];
    // test "no free outgoing ids"
    for _ in 0..2u8.pow(7) {
        streams.push(Stream::new(b"".to_vec(), PlayerScope::new(0)).unwrap());
    }
    assert!(Stream::new(b"".to_vec(), PlayerScope::new(0)).is_err());

    drop(streams);

    assert!(Stream::new(b"".to_vec(), PlayerScope::new(0)).is_ok());
}
