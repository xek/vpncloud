use crate::{
    config::Config,
    crypto::CryptoCore,
    engine::common::Hash,
    error::Error,
    table::ClaimTable,
    traffic::{TrafficEntry, TrafficStats},
    types::{Address, RangeList},
    util::{Duration, MsgBuffer, TimeSource},
};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    io::{self, Write},
    net::SocketAddr,
    sync::Arc,
};

use super::common::PeerData;

#[derive(Clone)]
pub struct SharedPeerCrypto {
    peers: Arc<Mutex<HashMap<SocketAddr, Option<Arc<CryptoCore>>, Hash>>>,
    cache: HashMap<SocketAddr, Option<Arc<CryptoCore>>, Hash>, //TODO: local hashmap as cache
}

impl SharedPeerCrypto {
    pub fn new() -> Self {
        SharedPeerCrypto { peers: Arc::new(Mutex::new(HashMap::default())), cache: HashMap::default() }
    }

    pub fn encrypt_for(&mut self, peer: SocketAddr, data: &mut MsgBuffer) -> Result<(), Error> {
        let crypto = match self.cache.get(&peer) {
            Some(crypto) => crypto,
            None => {
                let peers = self.peers.lock();
                if let Some(crypto) = peers.get(&peer) {
                    self.cache.insert(peer, crypto.clone());
                    self.cache.get(&peer).unwrap()
                } else {
                    return Err(Error::InvalidCryptoState("No crypto found for peer"));
                }
            }
        };
        if let Some(crypto) = crypto {
            crypto.encrypt(data);
        }
        Ok(())
    }

    pub fn store(&mut self, data: &HashMap<SocketAddr, PeerData, Hash>) {
        self.cache.clear();
        self.cache.extend(data.iter().map(|(k, v)| (*k, v.crypto.get_core())));
        let mut peers = self.peers.lock();
        peers.clear();
        peers.extend(self.cache.iter().map(|(k, v)| (*k, v.clone())));
    }

    pub fn load(&mut self) {
        let peers = self.peers.lock();
        self.cache.clear();
        self.cache.extend(peers.iter().map(|(k, v)| (*k, v.clone())));
    }

    pub fn get_snapshot(&mut self) -> &HashMap<SocketAddr, Option<Arc<CryptoCore>>, Hash> {
        &self.cache
    }

    pub fn count(&self) -> usize {
        self.cache.len()
    }
}

#[derive(Clone)]
pub struct SharedTraffic {
    traffic: Arc<Mutex<TrafficStats>>,
}

impl SharedTraffic {
    pub fn new() -> Self {
        Self { traffic: Arc::new(Mutex::new(Default::default())) }
    }

    pub fn sync(&mut self) {
        // TODO sync if needed
    }

    pub fn count_out_traffic(&self, peer: SocketAddr, bytes: usize) {
        self.traffic.lock().count_out_traffic(peer, bytes);
    }

    pub fn count_in_traffic(&self, peer: SocketAddr, bytes: usize) {
        self.traffic.lock().count_in_traffic(peer, bytes);
    }

    pub fn count_out_payload(&self, remote: Address, local: Address, bytes: usize) {
        self.traffic.lock().count_out_payload(remote, local, bytes);
    }

    pub fn count_in_payload(&self, remote: Address, local: Address, bytes: usize) {
        self.traffic.lock().count_in_payload(remote, local, bytes);
    }

    pub fn count_dropped_payload(&self, bytes: usize) {
        self.traffic.lock().count_dropped_payload(bytes);
    }

    pub fn count_invalid_protocol(&self, bytes: usize) {
        self.traffic.lock().count_invalid_protocol(bytes);
    }

    pub fn period(&mut self, cleanup_idle: Option<usize>) {
        self.traffic.lock().period(cleanup_idle)
    }

    pub fn write_out<W: Write>(&self, out: &mut W) -> Result<(), io::Error> {
        self.traffic.lock().write_out(out)
    }

    pub fn total_peer_traffic(&self) -> TrafficEntry {
        self.traffic.lock().total_peer_traffic()
    }

    pub fn total_payload_traffic(&self) -> TrafficEntry {
        self.traffic.lock().total_payload_traffic()
    }

    pub fn dropped(&self) -> TrafficEntry {
        self.traffic.lock().dropped.clone()
    }
}

#[derive(Clone)]
pub struct SharedTable<TS: TimeSource> {
    table: Arc<Mutex<ClaimTable<TS>>>,
    //TODO: local reader lookup table Addr => Option<SocketAddr>
    //TODO: local writer cache Addr => SocketAddr
}

impl<TS: TimeSource> SharedTable<TS> {
    pub fn new(config: &Config) -> Self {
        let table = ClaimTable::new(config.switch_timeout as Duration, config.peer_timeout as Duration);
        SharedTable { table: Arc::new(Mutex::new(table)) }
    }

    pub fn sync(&mut self) {
        // TODO sync if needed
        // once every x seconds
        // fetch reader cache
        // clear writer cache
    }

    pub fn lookup(&mut self, addr: Address) -> Option<SocketAddr> {
        // TODO: use local reader cache
        // if not found, use shared table and put into cache
        self.table.lock().lookup(addr)
    }

    pub fn set_claims(&mut self, peer: SocketAddr, claims: RangeList) {
        // clear writer cache
        self.table.lock().set_claims(peer, claims)
    }

    pub fn remove_claims(&mut self, peer: SocketAddr) {
        // clear writer cache
        self.table.lock().remove_claims(peer)
    }

    pub fn cache(&mut self, addr: Address, peer: SocketAddr) {
        // check writer cache and only write real updates to shared table
        self.table.lock().cache(addr, peer)
    }

    pub fn housekeep(&mut self) {
        self.table.lock().housekeep()
    }

    pub fn write_out<W: Write>(&self, out: &mut W) -> Result<(), io::Error> {
        //TODO: stats call
        self.table.lock().write_out(out)
    }

    pub fn cache_len(&self) -> usize {
        //TODO: stats call
        self.table.lock().cache_len()
    }

    pub fn claim_len(&self) -> usize {
        //TODO: stats call
        self.table.lock().claim_len()
    }
}