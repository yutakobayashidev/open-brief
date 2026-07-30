import 'package:flutter_secure_storage/flutter_secure_storage.dart';

class SavedConnection {
  const SavedConnection({required this.baseUrl, required this.token});

  final String baseUrl;
  final String token;
}

class ConnectionStore {
  ConnectionStore({FlutterSecureStorage? storage})
    : _storage = storage ?? const FlutterSecureStorage();

  static const _urlKey = 'openbrief.base_url';
  static const _tokenKey = 'openbrief.device_token';

  final FlutterSecureStorage _storage;

  Future<SavedConnection?> read() async {
    final values = await _storage.readAll();
    final url = values[_urlKey];
    final token = values[_tokenKey];
    if (url == null || token == null) {
      return null;
    }
    return SavedConnection(baseUrl: url, token: token);
  }

  Future<void> write(SavedConnection connection) async {
    await _storage.write(key: _urlKey, value: connection.baseUrl);
    await _storage.write(key: _tokenKey, value: connection.token);
  }

  Future<void> clear() async {
    await _storage.delete(key: _urlKey);
    await _storage.delete(key: _tokenKey);
  }
}
