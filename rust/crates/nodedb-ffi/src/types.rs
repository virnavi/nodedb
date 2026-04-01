use std::os::raw::c_char;

pub type DbHandle = u64;

#[repr(C)]
pub struct NodeDbError {
    pub code: i32,
    pub message: *mut c_char,
}

impl NodeDbError {
    pub fn none() -> Self {
        NodeDbError {
            code: 0,
            message: std::ptr::null_mut(),
        }
    }

    pub fn new(code: i32, msg: &str) -> Self {
        let c_msg = std::ffi::CString::new(msg).unwrap_or_default();
        NodeDbError {
            code,
            message: c_msg.into_raw(),
        }
    }
}

// Error codes
pub const ERR_NONE: i32 = 0;
pub const ERR_INVALID_HANDLE: i32 = 1;
pub const ERR_STORAGE: i32 = 2;
pub const ERR_SERIALIZATION: i32 = 3;
pub const ERR_NOT_FOUND: i32 = 4;
pub const ERR_INVALID_QUERY: i32 = 5;
pub const ERR_INTERNAL: i32 = 6;
pub const ERR_NULL_POINTER: i32 = 7;

// Graph error codes
pub const ERR_GRAPH_NODE_NOT_FOUND: i32 = 10;
pub const ERR_GRAPH_EDGE_NOT_FOUND: i32 = 11;
pub const ERR_GRAPH_DELETE_RESTRICTED: i32 = 12;
pub const ERR_GRAPH_TRAVERSAL: i32 = 13;

pub type GraphHandle = u64;
pub type VectorHandle = u64;

// Vector error codes
pub const ERR_VECTOR_NOT_FOUND: i32 = 20;
pub const ERR_VECTOR_DIMENSION_MISMATCH: i32 = 21;
pub const ERR_VECTOR_NOT_INITIALIZED: i32 = 22;
pub const ERR_VECTOR_SEARCH: i32 = 23;

pub type FederationHandle = u64;

// Federation error codes
pub const ERR_FEDERATION_PEER_NOT_FOUND: i32 = 30;
pub const ERR_FEDERATION_GROUP_NOT_FOUND: i32 = 31;
pub const ERR_FEDERATION_DUPLICATE_NAME: i32 = 32;
pub const ERR_FEDERATION_INVALID_MEMBER: i32 = 33;

pub type DacHandle = u64;

// DAC error codes
pub const ERR_DAC_RULE_NOT_FOUND: i32 = 40;
pub const ERR_DAC_INVALID_COLLECTION: i32 = 41;
pub const ERR_DAC_INVALID_DOCUMENT: i32 = 42;

pub type TransportHandle = u64;

// Transport error codes
pub const ERR_TRANSPORT_CONNECTION: i32 = 50;
pub const ERR_TRANSPORT_HANDSHAKE: i32 = 51;
pub const ERR_TRANSPORT_SEND: i32 = 52;
pub const ERR_TRANSPORT_TIMEOUT: i32 = 53;
pub const ERR_TRANSPORT_PEER_REJECTED: i32 = 54;
pub const ERR_TRANSPORT_CRYPTO: i32 = 55;
pub const ERR_PAIRING_REQUIRED: i32 = 150;
pub const ERR_PAIRING_VERIFICATION_FAILED: i32 = 151;
pub const ERR_PAIRING_NOT_FOUND: i32 = 152;
pub const ERR_PAIRING_ERROR: i32 = 153;

pub type ProvenanceHandle = u64;

// Provenance error codes
pub const ERR_PROVENANCE_NOT_FOUND: i32 = 60;
pub const ERR_PROVENANCE_INVALID_CONFIDENCE: i32 = 61;
pub const ERR_PROVENANCE_VERIFICATION: i32 = 62;
pub const ERR_PROVENANCE_CANONICAL: i32 = 63;

pub type KeyResolverHandle = u64;

// Key resolver error codes
pub const ERR_KEYRESOLVER_NOT_FOUND: i32 = 70;
pub const ERR_KEYRESOLVER_INVALID_HEX: i32 = 71;
pub const ERR_KEYRESOLVER_EXPIRED: i32 = 72;
pub const ERR_KEYRESOLVER_ENTRY_NOT_FOUND: i32 = 73;

pub type AiProvenanceHandle = u64;

// AI provenance error codes
pub const ERR_AI_PROVENANCE_ENVELOPE_NOT_FOUND: i32 = 80;
pub const ERR_AI_PROVENANCE_INVALID_CONFIDENCE: i32 = 81;
pub const ERR_AI_PROVENANCE_COLLECTION_NOT_ENABLED: i32 = 82;
pub const ERR_AI_PROVENANCE_CONFIG: i32 = 83;

pub type AiQueryHandle = u64;

// AI query error codes
pub const ERR_AI_QUERY_SCHEMA_VALIDATION: i32 = 90;
pub const ERR_AI_QUERY_CONFIDENCE_BELOW_THRESHOLD: i32 = 91;
pub const ERR_AI_QUERY_COLLECTION_NOT_ENABLED: i32 = 92;
pub const ERR_AI_QUERY_CONFIG: i32 = 93;
pub const ERR_AI_QUERY_NOSQL: i32 = 94;

// Trigger error codes
pub const ERR_TRIGGER_ABORT: i32 = 100;
pub const ERR_TRIGGER_NOT_FOUND: i32 = 101;

// Singleton error codes
pub const ERR_SINGLETON_DELETE: i32 = 110;
pub const ERR_SINGLETON_CLEAR: i32 = 111;

// Preference error codes
pub const ERR_PREFERENCE_NOT_FOUND: i32 = 120;
pub const ERR_PREFERENCE_ERROR: i32 = 121;

// Reserved schema error codes
pub const ERR_RESERVED_SCHEMA_WRITE: i32 = 130;

// Access history & trim error codes
pub const ERR_ACCESS_HISTORY: i32 = 140;
pub const ERR_TRIM_NEVER_TRIM: i32 = 141;
pub const ERR_TRIM_POLICY_INVALID: i32 = 142;
pub const ERR_TRIM_ABORTED: i32 = 143;

// Cache error codes
pub const ERR_CACHE_CONFIG_INVALID: i32 = 150;
