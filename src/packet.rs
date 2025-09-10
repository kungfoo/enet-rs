use enet_sys::{
    _ENetPacketFlag_ENET_PACKET_FLAG_NO_ALLOCATE, _ENetPacketFlag_ENET_PACKET_FLAG_RELIABLE,
    _ENetPacketFlag_ENET_PACKET_FLAG_UNRELIABLE_FRAGMENT,
    _ENetPacketFlag_ENET_PACKET_FLAG_UNSEQUENCED, ENetPacket, enet_packet_create,
    enet_packet_destroy,
};

use crate::Error;

/// A packet that can be sent or retrieved on an ENet-connection.
#[derive(Debug)]
pub struct Packet {
    inner: *mut ENetPacket,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
/// Mode that can be set when transmitting a packet.
///
/// ENet does not support reliable but unsequenced packets.
pub enum PacketMode {
    /// The packet will be sent unreliably but sequenced (ENet default).
    UnreliableSequenced,
    /// The packet will be sent unreliably and unsequenced.
    UnreliableUnsequenced,
    /// The packet will be sent unreliably, unsequenced and in case it exceeds the MTU it will be fragmented using unreliable sends.
    UnreliableUnsequencedUnreliablyFragmented,
    /// The packet will be sent reliably and sequenced with other reliable
    /// packets.
    ReliableSequenced,
}

impl PacketMode {
    /// Returns whether this represents a reliable mode.
    pub fn is_reliable(&self) -> bool {
        match self {
            PacketMode::UnreliableSequenced => false,
            PacketMode::UnreliableUnsequenced => false,
            PacketMode::UnreliableUnsequencedUnreliablyFragmented => false,
            PacketMode::ReliableSequenced => true,
        }
    }

    /// Returns whether this represents a sequenced mode.
    pub fn is_sequenced(&self) -> bool {
        match self {
            PacketMode::UnreliableSequenced => true,
            PacketMode::UnreliableUnsequenced => false,
            PacketMode::UnreliableUnsequencedUnreliablyFragmented => false,
            PacketMode::ReliableSequenced => true,
        }
    }

    fn to_sys_flags(self) -> u32 {
        match self {
            PacketMode::UnreliableSequenced => 0,
            PacketMode::UnreliableUnsequenced => _ENetPacketFlag_ENET_PACKET_FLAG_UNSEQUENCED,
            PacketMode::ReliableSequenced => _ENetPacketFlag_ENET_PACKET_FLAG_RELIABLE,
            PacketMode::UnreliableUnsequencedUnreliablyFragmented => {
                _ENetPacketFlag_ENET_PACKET_FLAG_UNSEQUENCED
                    | _ENetPacketFlag_ENET_PACKET_FLAG_UNRELIABLE_FRAGMENT
            }
        }
    }
}

impl Packet {
    /// Creates a new Packet with optional reliability settings.
    ///
    /// The data is consumed and moved into the enet package without copy.
    pub fn new(data: Vec<u8>, mode: PacketMode) -> Result<Packet, Error> {
        let res = unsafe {
            enet_packet_create(
                data.as_ptr() as *const _,
                data.len(),
                mode.to_sys_flags() | _ENetPacketFlag_ENET_PACKET_FLAG_NO_ALLOCATE,
            )
        };

        if res.is_null() {
            return Err(Error(0));
        }

        unsafe {
            (*res).userData = data.capacity() as *mut _;
            (*res).freeCallback = Some(packet_free_callback);
        }

        // we leak the data here, as the enet package has taken ownership of it.
        // The data will be deallocated by packet_free_callback
        std::mem::forget(data);

        Ok(Packet::from_sys_packet(res))
    }

    pub(crate) fn from_sys_packet(inner: *mut ENetPacket) -> Packet {
        Packet { inner }
    }

    /// Does NOT run this `Packet`'s destructor.
    pub(crate) fn into_inner(self) -> *mut ENetPacket {
        let res = self.inner;
        std::mem::forget(self);
        res
    }

    /// Returns a reference to the bytes inside this packet.
    pub fn data(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts((*self.inner).data, (*self.inner).dataLength) }
    }
}

impl Drop for Packet {
    fn drop(&mut self) {
        unsafe {
            enet_packet_destroy(self.inner);
        }
    }
}

unsafe extern "C" fn packet_free_callback(packet: *mut ENetPacket) {
    drop(Vec::<u8>::from_raw_parts(
        (*packet).data,
        (*packet).dataLength,
        (*packet).userData as usize,
    ));
}
