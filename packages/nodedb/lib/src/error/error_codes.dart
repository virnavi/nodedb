/// Error codes mirroring rust/crates/nodedb-ffi/src/types.rs
library;

// General
const errNone = 0;
const errInvalidHandle = 1;
const errStorage = 2;
const errSerialization = 3;
const errNotFound = 4;
const errInvalidQuery = 5;
const errInternal = 6;
const errNullPointer = 7;

// Graph (10-13)
const errGraphNodeNotFound = 10;
const errGraphEdgeNotFound = 11;
const errGraphDeleteRestricted = 12;
const errGraphTraversal = 13;

// Vector (20-23)
const errVectorNotFound = 20;
const errVectorDimensionMismatch = 21;
const errVectorNotInitialized = 22;
const errVectorSearch = 23;

// Federation (30-33)
const errFederationPeerNotFound = 30;
const errFederationGroupNotFound = 31;
const errFederationDuplicateName = 32;
const errFederationInvalidMember = 33;

// DAC (40-42)
const errDacRuleNotFound = 40;
const errDacInvalidCollection = 41;
const errDacInvalidDocument = 42;

// Transport (50-55)
const errTransportConnection = 50;
const errTransportHandshake = 51;
const errTransportSend = 52;
const errTransportTimeout = 53;
const errTransportPeerRejected = 54;
const errTransportCrypto = 55;

// Provenance (60-63)
const errProvenanceNotFound = 60;
const errProvenanceInvalidConfidence = 61;
const errProvenanceVerification = 62;
const errProvenanceCanonical = 63;

// Key Resolver (70-73)
const errKeyresolverNotFound = 70;
const errKeyresolverInvalidHex = 71;
const errKeyresolverExpired = 72;
const errKeyresolverEntryNotFound = 73;

// AI Provenance (80-83)
const errAiProvenanceEnvelopeNotFound = 80;
const errAiProvenanceInvalidConfidence = 81;
const errAiProvenanceCollectionNotEnabled = 82;
const errAiProvenanceConfig = 83;

// AI Query (90-94)
const errAiQuerySchemaValidation = 90;
const errAiQueryConfidenceBelowThreshold = 91;
const errAiQueryCollectionNotEnabled = 92;
const errAiQueryConfig = 93;
const errAiQueryNosql = 94;

// Trigger (100-101)
const errTriggerAbort = 100;
const errTriggerNotFound = 101;

// Singleton (110-111)
const errSingletonDelete = 110;
const errSingletonClear = 111;

// Preference (120-121)
const errPreferenceNotFound = 120;
const errPreferenceError = 121;

// Reserved schema (130)
const errReservedSchemaWrite = 130;
