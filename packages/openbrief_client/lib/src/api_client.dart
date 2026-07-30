import 'dart:async';
import 'dart:convert';

import 'package:http/http.dart' as http;
import 'package:web_socket_channel/io.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import 'domain.dart';

class OpenBriefApi {
  static const eventSubprotocol = 'openbrief.events.v1';

  OpenBriefApi({
    required String baseUrl,
    required this.token,
    http.Client? client,
  }) : baseUrl = normalizeBaseUrl(baseUrl),
       _client = client ?? http.Client();

  final String baseUrl;
  final String token;
  final http.Client _client;

  Future<RemoteSnapshot> snapshot() async {
    final data = await _request('GET', '/v1/snapshot');
    return RemoteSnapshot.fromJson(data);
  }

  Future<AgentStatus> startAgent() async {
    final data = await _request('PUT', '/v1/agent-session');
    return AgentStatus.fromJson(data);
  }

  Future<String> startTurn(String text) async {
    final data = await _request('POST', '/v1/turns', body: {'text': text});
    return data['id'] as String;
  }

  Future<TriageProposal> confirmProposal(String proposalId) async {
    final encoded = Uri.encodeComponent(proposalId);
    final data = await _request('POST', '/v1/proposals/$encoded/confirmations');
    return TriageProposal.fromJson(data);
  }

  Future<WebSocketChannel> events(int after) async {
    final base = Uri.parse(baseUrl);
    final uri = base.replace(
      scheme: base.scheme == 'https' ? 'wss' : 'ws',
      path: '${base.path}/v1/events'.replaceAll('//', '/'),
      queryParameters: {'after': '$after'},
    );
    final channel = IOWebSocketChannel.connect(
      uri,
      headers: {'Authorization': 'Bearer $token'},
      protocols: [eventSubprotocol],
      connectTimeout: const Duration(seconds: 10),
    );
    await channel.ready;
    return channel;
  }

  Future<JsonMap> _request(String method, String path, {JsonMap? body}) async {
    final uri = Uri.parse('$baseUrl$path');
    final headers = {
      'Authorization': 'Bearer $token',
      'Accept': 'application/json',
      if (body != null) 'Content-Type': 'application/json',
    };
    final request = http.Request(method, uri)
      ..headers.addAll(headers)
      ..body = body == null ? '' : jsonEncode(body);
    final response = await _client
        .send(request)
        .timeout(const Duration(seconds: 15));
    final text = await response.stream.bytesToString();
    final decoded = text.isEmpty ? <String, dynamic>{} : jsonDecode(text);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      final error = decoded is JsonMap ? decoded : <String, dynamic>{};
      throw OpenBriefApiException(
        statusCode: response.statusCode,
        code: error['code'] as String? ?? 'request_failed',
        message: error['message'] as String? ?? 'OpenBriefへ接続できません',
      );
    }
    if (decoded is! JsonMap || decoded['data'] is! JsonMap) {
      throw const FormatException('OpenBrief API response is invalid');
    }
    return decoded['data'] as JsonMap;
  }

  void close() => _client.close();

  static String normalizeBaseUrl(String value) {
    final trimmed = value.trim().replaceFirst(RegExp(r'/+$'), '');
    final uri = Uri.tryParse(trimmed);
    if (uri == null || !uri.hasAuthority) {
      throw const FormatException('接続先URLを入力してください');
    }
    final local =
        uri.host == 'localhost' || uri.host == '127.0.0.1' || uri.host == '::1';
    if (uri.scheme != 'https' && !(local && uri.scheme == 'http')) {
      throw const FormatException('HTTPSの接続先を使用してください');
    }
    if (uri.path != '' && uri.path != '/') {
      throw const FormatException('接続先URLにはpathを含めないでください');
    }
    return trimmed;
  }
}

class OpenBriefApiException implements Exception {
  const OpenBriefApiException({
    required this.statusCode,
    required this.code,
    required this.message,
  });

  final int statusCode;
  final String code;
  final String message;

  @override
  String toString() => message;
}
