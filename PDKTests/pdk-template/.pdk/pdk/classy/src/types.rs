// Copyright 2023 Salesforce, Inc. All rights reserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootCid(u32);

impl From<RootCid> for u32 {
    fn from(id: RootCid) -> Self {
        id.0
    }
}

impl From<u32> for RootCid {
    fn from(id: u32) -> Self {
        RootCid(id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HttpCid(u32);

impl From<HttpCid> for u32 {
    fn from(id: HttpCid) -> Self {
        id.0
    }
}

impl From<u32> for HttpCid {
    fn from(id: u32) -> Self {
        HttpCid(id)
    }
}

// Context ID
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cid {
    Root(RootCid),
    Http(HttpCid),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestId(u32);

impl From<RequestId> for u32 {
    fn from(id: RequestId) -> Self {
        id.0
    }
}

impl From<u32> for RequestId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<Cid> for u32 {
    fn from(cid: Cid) -> Self {
        match cid {
            Cid::Root(id) => id.into(),
            Cid::Http(id) => id.into(),
        }
    }
}

impl From<HttpCid> for Cid {
    fn from(id: HttpCid) -> Self {
        Cid::Http(id)
    }
}

impl From<RootCid> for Cid {
    fn from(id: RootCid) -> Self {
        Cid::Root(id)
    }
}
