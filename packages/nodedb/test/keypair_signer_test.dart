import 'dart:io';

import 'package:nodedb/nodedb.dart';
import 'package:nodedb_ffi/nodedb_ffi.dart';
import 'package:test/test.dart';

void main() {
  late NodeDbBindings bindings;
  late Directory tempDir;

  setUpAll(() {
    bindings = NodeDbBindings(loadNodeDbLibrary());
  });

  setUp(() {
    tempDir = Directory.systemTemp.createTempSync('nodedb_keypair_test_');
  });

  tearDown(() {
    tempDir.deleteSync(recursive: true);
  });

  group('OwnerKeyStatus', () {
    test('fromString parses all variants', () {
      expect(OwnerKeyStatus.fromString('verified'), OwnerKeyStatus.verified);
      expect(OwnerKeyStatus.fromString('mismatch'), OwnerKeyStatus.mismatch);
      expect(OwnerKeyStatus.fromString('unbound'), OwnerKeyStatus.unbound);
      expect(OwnerKeyStatus.fromString('unknown'), OwnerKeyStatus.unbound);
    });
  });

  group('Keypair', () {
    test('generateKeypair returns valid 64-char hex keys', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);
      final keypair = engine.generateKeypair();

      expect(keypair.privateKeyHex.length, 64);
      expect(keypair.publicKeyHex.length, 64);
      // Valid hex chars
      expect(RegExp(r'^[0-9a-f]+$').hasMatch(keypair.privateKeyHex), isTrue);
      expect(RegExp(r'^[0-9a-f]+$').hasMatch(keypair.publicKeyHex), isTrue);

      engine.close();
    });

    test('generateKeypair returns unique keys each time', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);
      final kp1 = engine.generateKeypair();
      final kp2 = engine.generateKeypair();

      expect(kp1.privateKeyHex, isNot(equals(kp2.privateKeyHex)));
      expect(kp1.publicKeyHex, isNot(equals(kp2.publicKeyHex)));

      engine.close();
    });

    test('ownerKeyStatusTyped returns unbound for new db', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);
      expect(engine.ownerKeyStatusTyped, OwnerKeyStatus.unbound);
      engine.close();
    });

    test('ownerKeyStatusTyped returns verified when opened with key', () {
      // First open to generate a keypair
      final engine1 = NoSqlEngine.open(bindings, tempDir.path);
      final keypair = engine1.generateKeypair();
      engine1.close();

      // Open a new database with the keypair
      final dir2 = Directory('${tempDir.path}/bound')..createSync();
      final engine2 = NoSqlEngine.open(
        bindings,
        dir2.path,
        ownerPrivateKeyHex: keypair.privateKeyHex,
      );
      expect(engine2.ownerKeyStatusTyped, OwnerKeyStatus.verified);
      engine2.close();
    });

    test('rotateOwnerKey succeeds with correct current key', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);
      final kp1 = engine.generateKeypair();
      engine.close();

      // Open with keypair
      final engine2 = NoSqlEngine.open(
        bindings,
        '${tempDir.path}/rotate',
        ownerPrivateKeyHex: kp1.privateKeyHex,
      );
      expect(engine2.ownerKeyStatusTyped, OwnerKeyStatus.verified);

      // Generate new keypair and rotate
      final kp2 = engine2.generateKeypair();
      final result = engine2.rotateOwnerKey(
        kp1.privateKeyHex,
        kp2.privateKeyHex,
      );

      expect(result.status, 'rotated');
      expect(result.newFingerprint.length, 64);
      engine2.close();
    });
  });

  group('Signing', () {
    test('signData produces non-empty hex signature', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);
      final keypair = engine.generateKeypair();

      final signature = engine.signData(
        keypair.privateKeyHex,
        'test payload',
      );

      // Ed25519 signature = 64 bytes = 128 hex chars
      expect(signature.length, 128);
      expect(RegExp(r'^[0-9a-f]+$').hasMatch(signature), isTrue);

      engine.close();
    });
  });

  group('NodeDBSigner', () {
    test('computeHash returns hex hash', () {
      final nosql = NoSqlEngine.open(bindings, tempDir.path);
      final provenance = ProvenanceEngine.open(
        bindings,
        '${tempDir.path}/prov',
      );
      final keypair = nosql.generateKeypair();

      final signer = NodeDBSigner(
        nosql: nosql,
        provenance: provenance,
        keypair: keypair,
        userId: 'test-user',
      );

      final hash = signer.computeHash({'name': 'Alice', 'age': 30});
      expect(hash.length, 64); // SHA-256 hex
      expect(RegExp(r'^[0-9a-f]+$').hasMatch(hash), isTrue);

      provenance.close();
      nosql.close();
    });

    test('attachSigned creates envelope with signature', () {
      final nosql = NoSqlEngine.open(bindings, tempDir.path);
      final provDir = Directory('${tempDir.path}/prov')..createSync();
      final provenance = ProvenanceEngine.open(bindings, provDir.path);
      final keypair = nosql.generateKeypair();

      final signer = NodeDBSigner(
        nosql: nosql,
        provenance: provenance,
        keypair: keypair,
        userId: 'test-user',
      );

      // Write a document first
      nosql.writeTxn([WriteOp.put('users', data: {'name': 'Alice'})]);

      final envelope = signer.attachSigned(
        collection: 'users',
        recordId: 1,
        data: {'name': 'Alice'},
        sourceId: 'user:direct',
      );

      expect(envelope.pkiSignature, isNotNull);
      expect(envelope.pkiSignature!.length, 128);
      expect(envelope.pkiId, keypair.publicKeyHex);
      expect(envelope.userId, 'test-user');
      expect(envelope.contentHash.length, 64);

      provenance.close();
      nosql.close();
    });

    test('signed envelope verifies with correct public key', () {
      final nosql = NoSqlEngine.open(bindings, tempDir.path);
      final provDir = Directory('${tempDir.path}/prov')..createSync();
      final provenance = ProvenanceEngine.open(bindings, provDir.path);
      final keypair = nosql.generateKeypair();

      final signer = NodeDBSigner(
        nosql: nosql,
        provenance: provenance,
        keypair: keypair,
        userId: 'test-user',
      );

      final envelope = signer.attachSigned(
        collection: 'users',
        recordId: 1,
        data: {'name': 'Alice'},
        sourceId: 'user:direct',
      );

      // Verify the envelope using the public key
      final verified = provenance.verify(
        envelope.id,
        keypair.publicKeyHex,
      );

      expect(verified.verificationStatus, 'verified');

      provenance.close();
      nosql.close();
    });
  });

  group('NodeDB facade', () {
    test('generateKeypair delegates', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
      );

      final keypair = db.generateKeypair();
      expect(keypair.privateKeyHex.length, 64);
      expect(keypair.publicKeyHex.length, 64);

      db.close();
    });

    test('ownerKeyStatusTyped delegates', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
      );

      expect(db.ownerKeyStatusTyped, OwnerKeyStatus.unbound);
      db.close();
    });
  });
}
