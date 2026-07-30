import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:openbrief_client/openbrief_client.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import 'connection_store.dart';

enum AppPhase { restoring, disconnected, connecting, connected }

abstract class OpenBriefController extends ChangeNotifier {
  AppPhase get phase;
  Brief? get brief;
  TriageProposal? get returnThread;
  TriageProposal? get proposal;
  AgentStatus? get agent;
  List<ConversationMessage> get messages;
  bool get live;
  bool get working;
  String? get error;
  String? get baseUrl;

  Future<void> connect(String baseUrl, String token);
  Future<void> refresh();
  Future<void> startAgent();
  Future<void> send(String text);
  Future<void> confirmProposal();
  Future<void> forgetConnection();
  void clearError();
}

class LiveOpenBriefController extends OpenBriefController {
  LiveOpenBriefController({ConnectionStore? store})
    : _store = store ?? ConnectionStore() {
    unawaited(_restore());
  }

  final ConnectionStore _store;
  OpenBriefApi? _api;
  WebSocketChannel? _events;
  StreamSubscription<dynamic>? _eventSubscription;

  AppPhase _phase = AppPhase.restoring;
  Brief? _brief;
  TriageProposal? _returnThread;
  TriageProposal? _proposal;
  AgentStatus? _agent;
  final List<ConversationMessage> _messages = [];
  bool _live = false;
  bool _working = false;
  String? _error;
  String? _baseUrl;
  int _cursor = 0;

  @override
  AppPhase get phase => _phase;
  @override
  Brief? get brief => _brief;
  @override
  TriageProposal? get returnThread => _returnThread;
  @override
  TriageProposal? get proposal => _proposal;
  @override
  AgentStatus? get agent => _agent;
  @override
  List<ConversationMessage> get messages => List.unmodifiable(_messages);
  @override
  bool get live => _live;
  @override
  bool get working => _working;
  @override
  String? get error => _error;
  @override
  String? get baseUrl => _baseUrl;

  Future<void> _restore() async {
    try {
      final saved = await _store.read();
      if (saved == null) {
        _phase = AppPhase.disconnected;
        notifyListeners();
        return;
      }
      await connect(saved.baseUrl, saved.token);
    } catch (exception) {
      _phase = AppPhase.disconnected;
      _error = _message(exception);
      notifyListeners();
    }
  }

  @override
  Future<void> connect(String baseUrl, String token) async {
    _phase = AppPhase.connecting;
    _working = true;
    _error = null;
    notifyListeners();
    OpenBriefApi? candidate;
    try {
      candidate = OpenBriefApi(baseUrl: baseUrl, token: token.trim());
      final snapshot = await candidate.snapshot();
      await _replaceApi(candidate, snapshot);
      candidate = null;
      await _store.write(
        SavedConnection(baseUrl: _baseUrl!, token: token.trim()),
      );
      _phase = AppPhase.connected;
    } catch (exception) {
      candidate?.close();
      _phase = AppPhase.disconnected;
      _error = _message(exception);
    } finally {
      _working = false;
      notifyListeners();
    }
  }

  Future<void> _replaceApi(
    OpenBriefApi candidate,
    RemoteSnapshot snapshot,
  ) async {
    await _closeTransport();
    _api?.close();
    _api = candidate;
    _baseUrl = candidate.baseUrl;
    _applySnapshot(snapshot);
    try {
      _events = await candidate.events(_cursor);
      _eventSubscription = _events!.stream.listen(
        _onRawEvent,
        onError: (Object exception) {
          _live = false;
          _error = 'ライブ更新が切れました。更新ボタンで再接続できます。';
          notifyListeners();
        },
        onDone: () {
          _live = false;
          notifyListeners();
        },
      );
      _live = true;
    } catch (_) {
      _live = false;
      _error = 'Briefは読めますが、ライブ更新へ接続できません。';
    }
  }

  void _applySnapshot(RemoteSnapshot snapshot) {
    _brief = snapshot.brief;
    _returnThread = snapshot.returnThread;
    _proposal = snapshot.pendingProposal;
    _agent = snapshot.agent;
    _cursor = snapshot.nextSequence;
  }

  @override
  Future<void> refresh() async {
    final api = _api;
    if (api == null || _working) return;
    _working = true;
    _error = null;
    notifyListeners();
    try {
      final snapshot = await api.snapshot();
      _applySnapshot(snapshot);
      if (!_live) {
        await _eventSubscription?.cancel();
        _events = await api.events(_cursor);
        _eventSubscription = _events!.stream.listen(
          _onRawEvent,
          onError: (_) {
            _live = false;
            notifyListeners();
          },
          onDone: () {
            _live = false;
            notifyListeners();
          },
        );
        _live = true;
      }
    } catch (exception) {
      _error = _message(exception);
    } finally {
      _working = false;
      notifyListeners();
    }
  }

  @override
  Future<void> startAgent() async {
    final api = _api;
    if (api == null || _working) return;
    _working = true;
    _error = null;
    notifyListeners();
    try {
      _agent = await api.startAgent();
    } catch (exception) {
      _error = _message(exception);
    } finally {
      _working = false;
      notifyListeners();
    }
  }

  @override
  Future<void> send(String text) async {
    final api = _api;
    final value = text.trim();
    if (api == null || value.isEmpty || _working) return;
    _working = true;
    _error = null;
    _messages.add(
      ConversationMessage(
        id: 'user-${DateTime.now().microsecondsSinceEpoch}',
        role: 'user',
        text: value,
        streaming: false,
      ),
    );
    notifyListeners();
    try {
      await api.startTurn(value);
      if (!_live) {
        _working = false;
        notifyListeners();
      }
    } catch (exception) {
      _error = _message(exception);
      _working = false;
      notifyListeners();
    }
  }

  @override
  Future<void> confirmProposal() async {
    final api = _api;
    final pending = _proposal;
    if (api == null || pending == null || _working) return;
    _working = true;
    _error = null;
    notifyListeners();
    try {
      _returnThread = await api.confirmProposal(pending.id);
      _proposal = null;
    } catch (exception) {
      _error = _message(exception);
    } finally {
      _working = false;
      notifyListeners();
    }
  }

  void _onRawEvent(dynamic raw) {
    try {
      final event = SequencedEvent.fromJson(
        jsonDecode(raw as String) as JsonMap,
      );
      _cursor = event.sequence + 1;
      final payload = event.payload;
      switch (event.type) {
        case 'agent_status_changed':
          _agent = AgentStatus.fromJson(payload['status'] as JsonMap);
          break;
        case 'message_started':
          _messages.add(
            ConversationMessage(
              id: payload['id'] as String,
              role: payload['role'] as String,
              text: '',
              streaming: true,
            ),
          );
          break;
        case 'message_delta':
          final id = payload['id'] as String;
          final index = _messages.indexWhere((message) => message.id == id);
          if (index >= 0) {
            _messages[index] = _messages[index].copyWith(
              text: '${_messages[index].text}${payload['text'] as String}',
            );
          }
          break;
        case 'message_finished':
          final id = payload['id'] as String;
          final index = _messages.indexWhere((message) => message.id == id);
          if (index >= 0) {
            _messages[index] = _messages[index].copyWith(streaming: false);
          }
          break;
        case 'proposal_received':
          _proposal = TriageProposal.fromJson(payload['proposal'] as JsonMap);
          break;
        case 'brief_changed':
          _brief = Brief.fromJson(payload['brief'] as JsonMap);
          break;
        case 'proposal_applied':
          _returnThread = TriageProposal.fromJson(
            payload['proposal'] as JsonMap,
          );
          _proposal = null;
          break;
        case 'turn_finished':
          _working = false;
          break;
        case 'error':
          _working = false;
          _error = payload['message'] as String;
          break;
      }
      notifyListeners();
    } catch (_) {
      _error = 'ライブ更新を読み取れませんでした。';
      notifyListeners();
    }
  }

  @override
  Future<void> forgetConnection() async {
    await _closeTransport();
    await _store.clear();
    _api?.close();
    _api = null;
    _phase = AppPhase.disconnected;
    _brief = null;
    _returnThread = null;
    _proposal = null;
    _agent = null;
    _messages.clear();
    _baseUrl = null;
    _live = false;
    _error = null;
    notifyListeners();
  }

  Future<void> _closeTransport() async {
    await _eventSubscription?.cancel();
    _eventSubscription = null;
    await _events?.sink.close();
    _events = null;
    _live = false;
  }

  @override
  void clearError() {
    _error = null;
    notifyListeners();
  }

  String _message(Object exception) {
    if (exception is OpenBriefApiException) return exception.message;
    if (exception is FormatException) return exception.message;
    if (exception is TimeoutException) return 'OpenBriefから応答がありません。';
    return 'OpenBriefへ接続できません。URLとtokenを確認してください。';
  }

  @override
  void dispose() {
    unawaited(_closeTransport());
    _api?.close();
    super.dispose();
  }
}
