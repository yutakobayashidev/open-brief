import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:openbrief_client/openbrief_client.dart';
import 'package:openbrief_mobile/app_controller.dart';
import 'package:openbrief_mobile/main.dart';

void main() {
  testWidgets('shows a finite brief and return thread', (tester) async {
    final controller = FakeController();

    await tester.pumpWidget(OpenBriefApp(controller: controller));

    expect(find.text('認証テストへ戻る'), findsOneWidget);
    expect(find.text('今、見るもの'), findsOneWidget);
    expect(find.text('返信待ちのメール'), findsOneWidget);
    expect(find.text('1件で止めています'), findsOneWidget);
    expect(find.text('Agentに相談できます'), findsOneWidget);
  });

  testWidgets('shows connection form while disconnected', (tester) async {
    final controller = FakeController()..currentPhase = AppPhase.disconnected;

    await tester.pumpWidget(OpenBriefApp(controller: controller));

    expect(find.byKey(const Key('base-url')), findsOneWidget);
    expect(find.byKey(const Key('device-token')), findsOneWidget);
    expect(find.byKey(const Key('connect')), findsOneWidget);
  });
}

class FakeController extends OpenBriefController {
  AppPhase currentPhase = AppPhase.connected;

  @override
  AppPhase get phase => currentPhase;
  @override
  Brief? get brief => const Brief(
    protect: [
      BriefItem(
        id: 'mail-1',
        title: '返信待ちのメール',
        reason: '相手が返答を待っています',
        source: 'gmail',
        observedAt: '2026-07-31T09:00:00+09:00',
      ),
    ],
    explore: [],
    coverage: [],
    generatedAt: '2026-07-31T09:00:00+09:00',
  );
  @override
  TriageProposal? get returnThread => const TriageProposal(
    id: 'proposal-1',
    summary: 'メールを扱ってから戻る',
    protectIds: ['mail-1'],
    exploreId: null,
    returnAnchor: '認証テストへ戻る',
    returnCommand: 'cargo test',
  );
  @override
  TriageProposal? get proposal => null;
  @override
  AgentStatus? get agent => const AgentStatus(
    availability: 'available',
    authentication: 'authenticated',
    process: 'ready',
    message: null,
    authMethods: [],
  );
  @override
  List<ConversationMessage> get messages => const [];
  @override
  bool get live => true;
  @override
  bool get working => false;
  @override
  String? get error => null;
  @override
  String? get baseUrl => 'https://openbrief.example';

  @override
  Future<void> connect(String baseUrl, String token) async {}
  @override
  Future<void> confirmProposal() async {}
  @override
  Future<void> forgetConnection() async {}
  @override
  Future<void> refresh() async {}
  @override
  Future<void> send(String text) async {}
  @override
  Future<void> startAgent() async {}
  @override
  void clearError() {}
}
