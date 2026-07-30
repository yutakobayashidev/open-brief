typedef JsonMap = Map<String, dynamic>;

class BriefItem {
  const BriefItem({
    required this.id,
    required this.title,
    required this.reason,
    required this.source,
    required this.observedAt,
    this.minutes,
  });

  factory BriefItem.fromJson(JsonMap json) => BriefItem(
    id: json['id'] as String,
    title: json['title'] as String,
    reason: json['reason'] as String,
    source: json['source'] as String,
    observedAt: json['observedAt'] as String,
    minutes: json['minutes'] as int?,
  );

  final String id;
  final String title;
  final String reason;
  final String source;
  final String observedAt;
  final int? minutes;
}

class Coverage {
  const Coverage({
    required this.source,
    required this.observedAt,
    required this.status,
  });

  factory Coverage.fromJson(JsonMap json) => Coverage(
    source: json['source'] as String,
    observedAt: json['observedAt'] as String,
    status: json['status'] as String,
  );

  final String source;
  final String observedAt;
  final String status;
}

class Brief {
  const Brief({
    required this.protect,
    required this.explore,
    required this.coverage,
    required this.generatedAt,
  });

  factory Brief.fromJson(JsonMap json) => Brief(
    protect: _maps(json['protect']).map(BriefItem.fromJson).toList(),
    explore: _maps(json['explore']).map(BriefItem.fromJson).toList(),
    coverage: _maps(json['coverage']).map(Coverage.fromJson).toList(),
    generatedAt: json['generatedAt'] as String,
  );

  final List<BriefItem> protect;
  final List<BriefItem> explore;
  final List<Coverage> coverage;
  final String generatedAt;

  int get length => protect.length + explore.length;
}

class TriageProposal {
  const TriageProposal({
    required this.id,
    required this.summary,
    required this.protectIds,
    required this.exploreId,
    required this.returnAnchor,
    required this.returnCommand,
  });

  factory TriageProposal.fromJson(JsonMap json) => TriageProposal(
    id: json['id'] as String,
    summary: json['summary'] as String,
    protectIds: (json['protectIds'] as List<dynamic>).cast<String>(),
    exploreId: json['exploreId'] as String?,
    returnAnchor: json['returnAnchor'] as String,
    returnCommand: json['returnCommand'] as String,
  );

  final String id;
  final String summary;
  final List<String> protectIds;
  final String? exploreId;
  final String returnAnchor;
  final String returnCommand;
}

class AgentStatus {
  const AgentStatus({
    required this.availability,
    required this.authentication,
    required this.process,
    required this.message,
    required this.authMethods,
  });

  factory AgentStatus.fromJson(JsonMap json) {
    final availability = json['availability'] as JsonMap;
    final authentication = json['authentication'] as JsonMap;
    final process = json['process'] as JsonMap;
    final methods = (authentication['methods'] as List<dynamic>? ?? const [])
        .cast<JsonMap>();
    return AgentStatus(
      availability: availability['status'] as String,
      authentication: authentication['status'] as String,
      process: process['status'] as String,
      message:
          process['message'] as String? ?? availability['message'] as String?,
      authMethods: methods
          .map(
            (method) => AuthMethod(
              id: method['id'] as String,
              name: method['name'] as String,
            ),
          )
          .toList(),
    );
  }

  final String availability;
  final String authentication;
  final String process;
  final String? message;
  final List<AuthMethod> authMethods;

  bool get ready =>
      availability == 'available' &&
      authentication == 'authenticated' &&
      (process == 'ready' || process == 'busy');

  bool get busy => process == 'busy' || process == 'starting';
}

class AuthMethod {
  const AuthMethod({required this.id, required this.name});

  final String id;
  final String name;
}

class RemoteSnapshot {
  const RemoteSnapshot({
    required this.brief,
    required this.returnThread,
    required this.pendingProposal,
    required this.agent,
    required this.nextSequence,
  });

  factory RemoteSnapshot.fromJson(JsonMap json) => RemoteSnapshot(
    brief: Brief.fromJson(json['brief'] as JsonMap),
    returnThread: json['returnThread'] == null
        ? null
        : TriageProposal.fromJson(json['returnThread'] as JsonMap),
    pendingProposal: json['pendingProposal'] == null
        ? null
        : TriageProposal.fromJson(json['pendingProposal'] as JsonMap),
    agent: AgentStatus.fromJson(json['agent'] as JsonMap),
    nextSequence: json['nextSequence'] as int,
  );

  final Brief brief;
  final TriageProposal? returnThread;
  final TriageProposal? pendingProposal;
  final AgentStatus agent;
  final int nextSequence;
}

class ConversationMessage {
  const ConversationMessage({
    required this.id,
    required this.role,
    required this.text,
    required this.streaming,
  });

  final String id;
  final String role;
  final String text;
  final bool streaming;

  ConversationMessage copyWith({String? text, bool? streaming}) =>
      ConversationMessage(
        id: id,
        role: role,
        text: text ?? this.text,
        streaming: streaming ?? this.streaming,
      );
}

class SequencedEvent {
  const SequencedEvent({
    required this.sequence,
    required this.type,
    required this.payload,
  });

  factory SequencedEvent.fromJson(JsonMap json) {
    final event = json['event'] as JsonMap;
    return SequencedEvent(
      sequence: json['sequence'] as int,
      type: event['type'] as String,
      payload: event,
    );
  }

  final int sequence;
  final String type;
  final JsonMap payload;
}

Iterable<JsonMap> _maps(dynamic value) =>
    (value as List<dynamic>).cast<JsonMap>();
