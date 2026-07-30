import 'package:flutter/material.dart';
import 'package:openbrief_client/openbrief_client.dart';

import 'app_controller.dart';

void main() => runApp(const OpenBriefApp());

class OpenBriefApp extends StatefulWidget {
  const OpenBriefApp({super.key, this.controller});

  final OpenBriefController? controller;

  @override
  State<OpenBriefApp> createState() => _OpenBriefAppState();
}

class _OpenBriefAppState extends State<OpenBriefApp> {
  late final OpenBriefController controller =
      widget.controller ?? LiveOpenBriefController();

  @override
  void dispose() {
    if (widget.controller == null) controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    const ink = Color(0xff17282c);
    const cloud = Color(0xfff1f5f4);
    const coral = Color(0xffe66b50);
    return MaterialApp(
      title: 'OpenBrief',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: coral,
          brightness: Brightness.light,
          surface: cloud,
          onSurface: ink,
        ),
        scaffoldBackgroundColor: cloud,
        textTheme: const TextTheme(
          displaySmall: TextStyle(
            fontFamily: 'serif',
            fontSize: 38,
            height: 1.08,
            fontWeight: FontWeight.w600,
            letterSpacing: -1.2,
          ),
          headlineSmall: TextStyle(
            fontFamily: 'serif',
            fontSize: 26,
            height: 1.2,
            fontWeight: FontWeight.w600,
          ),
          titleMedium: TextStyle(fontWeight: FontWeight.w700),
          bodyLarge: TextStyle(fontSize: 16, height: 1.55),
          bodyMedium: TextStyle(fontSize: 14, height: 1.5),
        ),
        inputDecorationTheme: const InputDecorationTheme(
          filled: true,
          fillColor: Colors.white,
          border: OutlineInputBorder(
            borderRadius: BorderRadius.all(Radius.circular(14)),
            borderSide: BorderSide.none,
          ),
        ),
        filledButtonTheme: FilledButtonThemeData(
          style: FilledButton.styleFrom(
            minimumSize: const Size(48, 52),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(14),
            ),
          ),
        ),
      ),
      home: AnimatedBuilder(
        animation: controller,
        builder: (context, _) => switch (controller.phase) {
          AppPhase.restoring => const _RestoringScreen(),
          AppPhase.disconnected ||
          AppPhase.connecting => _ConnectScreen(controller: controller),
          AppPhase.connected => _BriefScreen(controller: controller),
        },
      ),
    );
  }
}

class _RestoringScreen extends StatelessWidget {
  const _RestoringScreen();

  @override
  Widget build(BuildContext context) =>
      const Scaffold(body: Center(child: CircularProgressIndicator()));
}

class _ConnectScreen extends StatefulWidget {
  const _ConnectScreen({required this.controller});

  final OpenBriefController controller;

  @override
  State<_ConnectScreen> createState() => _ConnectScreenState();
}

class _ConnectScreenState extends State<_ConnectScreen> {
  final url = TextEditingController();
  final token = TextEditingController();

  @override
  void dispose() {
    url.dispose();
    token.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 480),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const _Wordmark(),
                  const SizedBox(height: 48),
                  Text('注意の続きを、\n手元に戻す。', style: theme.textTheme.displaySmall),
                  const SizedBox(height: 16),
                  Text(
                    'OpenBrief daemonへ安全に接続し、有限Briefと戻り先を確認します。',
                    style: theme.textTheme.bodyLarge,
                  ),
                  const SizedBox(height: 32),
                  TextField(
                    key: const Key('base-url'),
                    controller: url,
                    keyboardType: TextInputType.url,
                    autocorrect: false,
                    decoration: const InputDecoration(
                      labelText: '接続先',
                      hintText: 'https://openbrief.tailnet.ts.net',
                    ),
                  ),
                  const SizedBox(height: 14),
                  TextField(
                    key: const Key('device-token'),
                    controller: token,
                    obscureText: true,
                    autocorrect: false,
                    decoration: const InputDecoration(
                      labelText: 'Device token',
                    ),
                  ),
                  if (widget.controller.error case final error?) ...[
                    const SizedBox(height: 14),
                    _Notice(text: error),
                  ],
                  const SizedBox(height: 20),
                  FilledButton.icon(
                    key: const Key('connect'),
                    onPressed: widget.controller.working
                        ? null
                        : () => widget.controller.connect(url.text, token.text),
                    icon: widget.controller.working
                        ? const SizedBox.square(
                            dimension: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.arrow_forward_rounded),
                    label: const Text('OpenBriefへ接続'),
                  ),
                  const SizedBox(height: 16),
                  Text(
                    'tokenは端末の安全なストレージに保存されます。通常はTailscale Serve経由のHTTPSを使います。',
                    style: theme.textTheme.bodySmall,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _BriefScreen extends StatefulWidget {
  const _BriefScreen({required this.controller});

  final OpenBriefController controller;

  @override
  State<_BriefScreen> createState() => _BriefScreenState();
}

class _BriefScreenState extends State<_BriefScreen> {
  final composer = TextEditingController();

  @override
  void dispose() {
    composer.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final controller = widget.controller;
    return Scaffold(
      body: SafeArea(
        child: Column(
          children: [
            _Header(controller: controller),
            Expanded(
              child: RefreshIndicator(
                onRefresh: controller.refresh,
                child: ListView(
                  physics: const AlwaysScrollableScrollPhysics(),
                  padding: const EdgeInsets.fromLTRB(20, 8, 20, 132),
                  children: [
                    _ReturnThread(proposal: controller.returnThread),
                    if (controller.error case final error?) ...[
                      const SizedBox(height: 14),
                      _Notice(text: error, onClose: controller.clearError),
                    ],
                    const SizedBox(height: 28),
                    _BriefList(brief: controller.brief),
                    const SizedBox(height: 28),
                    _AgentPanel(
                      status: controller.agent,
                      working: controller.working,
                      onStart: controller.startAgent,
                    ),
                    if (controller.proposal case final proposal?) ...[
                      const SizedBox(height: 20),
                      _ProposalCard(
                        proposal: proposal,
                        working: controller.working,
                        onConfirm: controller.confirmProposal,
                      ),
                    ],
                    if (controller.messages.isNotEmpty) ...[
                      const SizedBox(height: 28),
                      const _SectionLabel('整理メモ'),
                      const SizedBox(height: 10),
                      ...controller.messages.take(6).map(_MessageCard.new),
                    ],
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
      bottomSheet: controller.phase == AppPhase.connected
          ? _Composer(
              controller: composer,
              enabled: controller.agent?.ready == true && !controller.working,
              onSend: () {
                final text = composer.text;
                composer.clear();
                controller.send(text);
              },
            )
          : null,
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.controller});

  final OpenBriefController controller;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(20, 10, 12, 8),
    child: Row(
      children: [
        const _Wordmark(),
        const Spacer(),
        _StatusChip(live: controller.live),
        IconButton(
          tooltip: '更新',
          onPressed: controller.working ? null : controller.refresh,
          icon: const Icon(Icons.refresh_rounded),
        ),
        PopupMenuButton<void>(
          tooltip: '接続設定',
          itemBuilder: (_) => [
            PopupMenuItem(
              onTap: controller.forgetConnection,
              child: const Text('接続情報を削除'),
            ),
          ],
        ),
      ],
    ),
  );
}

class _Wordmark extends StatelessWidget {
  const _Wordmark();

  @override
  Widget build(BuildContext context) => Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      Container(
        width: 11,
        height: 11,
        decoration: const BoxDecoration(
          color: Color(0xffe66b50),
          shape: BoxShape.circle,
        ),
      ),
      const SizedBox(width: 9),
      const Text(
        'OpenBrief',
        style: TextStyle(fontSize: 17, fontWeight: FontWeight.w800),
      ),
    ],
  );
}

class _StatusChip extends StatelessWidget {
  const _StatusChip({required this.live});

  final bool live;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 6),
    decoration: BoxDecoration(
      color: live ? const Color(0xffdbece7) : const Color(0xffffe5df),
      borderRadius: BorderRadius.circular(99),
    ),
    child: Text(
      live ? 'LIVE' : '更新待ち',
      style: const TextStyle(fontSize: 11, fontWeight: FontWeight.w800),
    ),
  );
}

class _ReturnThread extends StatelessWidget {
  const _ReturnThread({required this.proposal});

  final TriageProposal? proposal;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: const Color(0xff234e77),
        borderRadius: BorderRadius.circular(22),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 3,
            height: 72,
            decoration: BoxDecoration(
              color: const Color(0xff9fd1ca),
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(width: 16),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  'RETURN THREAD',
                  style: TextStyle(
                    color: Color(0xff9fd1ca),
                    fontSize: 11,
                    fontWeight: FontWeight.w800,
                    letterSpacing: 1.2,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  proposal?.returnAnchor ?? '戻り先はまだ決まっていません',
                  style: theme.textTheme.titleLarge?.copyWith(
                    color: Colors.white,
                    fontFamily: 'serif',
                  ),
                ),
                if (proposal?.returnCommand case final String command
                    when command.isNotEmpty) ...[
                  const SizedBox(height: 6),
                  Text(
                    command,
                    style: const TextStyle(color: Color(0xffdbe9ed)),
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _BriefList extends StatelessWidget {
  const _BriefList({required this.brief});

  final Brief? brief;

  @override
  Widget build(BuildContext context) {
    final value = brief;
    if (value == null) {
      return const Center(child: CircularProgressIndicator());
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const _SectionLabel('今、見るもの'),
        const SizedBox(height: 4),
        Text(
          '${value.length}件で止めています',
          style: Theme.of(context).textTheme.bodySmall,
        ),
        const SizedBox(height: 12),
        if (value.length == 0)
          const _EmptyCard()
        else ...[
          ...value.protect.map(
            (item) => _BriefCard(item: item, label: 'PROTECT'),
          ),
          ...value.explore.map(
            (item) => _BriefCard(item: item, label: 'EXPLORE'),
          ),
        ],
      ],
    );
  }
}

class _BriefCard extends StatelessWidget {
  const _BriefCard({required this.item, required this.label});

  final BriefItem item;
  final String label;

  @override
  Widget build(BuildContext context) => Container(
    margin: const EdgeInsets.only(bottom: 10),
    padding: const EdgeInsets.all(18),
    decoration: BoxDecoration(
      color: Colors.white,
      borderRadius: BorderRadius.circular(18),
      border: Border.all(color: const Color(0xffdce6e5)),
    ),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text(
              label,
              style: const TextStyle(
                color: Color(0xffd45c45),
                fontSize: 11,
                fontWeight: FontWeight.w800,
                letterSpacing: 1,
              ),
            ),
            const Spacer(),
            Text(item.source, style: Theme.of(context).textTheme.bodySmall),
          ],
        ),
        const SizedBox(height: 10),
        Text(item.title, style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 6),
        Text(item.reason),
        if (item.minutes case final minutes?) ...[
          const SizedBox(height: 8),
          Text('探索枠 $minutes分'),
        ],
      ],
    ),
  );
}

class _EmptyCard extends StatelessWidget {
  const _EmptyCard();

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.all(20),
    decoration: BoxDecoration(
      color: Colors.white,
      borderRadius: BorderRadius.circular(18),
    ),
    child: const Text('今すぐ注意を向ける候補はありません。追加で探さなくて大丈夫です。'),
  );
}

class _AgentPanel extends StatelessWidget {
  const _AgentPanel({
    required this.status,
    required this.working,
    required this.onStart,
  });

  final AgentStatus? status;
  final bool working;
  final VoidCallback onStart;

  @override
  Widget build(BuildContext context) {
    final ready = status?.ready == true;
    final auth = status?.authentication == 'required';
    return Container(
      padding: const EdgeInsets.all(18),
      decoration: BoxDecoration(
        color: const Color(0xffe3eeeb),
        borderRadius: BorderRadius.circular(18),
      ),
      child: Row(
        children: [
          Icon(
            ready ? Icons.check_circle_rounded : Icons.psychology_outlined,
            color: const Color(0xff234e77),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  ready
                      ? 'Agentに相談できます'
                      : auth
                      ? 'ホスト側でCodex認証が必要です'
                      : 'Agentはまだ休止中です',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                Text(status?.message ?? '必要なときだけ起動します'),
              ],
            ),
          ),
          if (!ready && !auth)
            TextButton(
              onPressed: working ? null : onStart,
              child: const Text('起動'),
            ),
        ],
      ),
    );
  }
}

class _ProposalCard extends StatelessWidget {
  const _ProposalCard({
    required this.proposal,
    required this.working,
    required this.onConfirm,
  });

  final TriageProposal proposal;
  final bool working;
  final VoidCallback onConfirm;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.all(20),
    decoration: BoxDecoration(
      color: const Color(0xffffeee9),
      borderRadius: BorderRadius.circular(20),
      border: Border.all(color: const Color(0xfff4b3a4)),
    ),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const _SectionLabel('この整理で進めますか？'),
        const SizedBox(height: 8),
        Text(proposal.summary),
        const SizedBox(height: 12),
        Text('戻り先: ${proposal.returnAnchor}'),
        const SizedBox(height: 16),
        FilledButton(
          onPressed: working ? null : onConfirm,
          child: const Text('本人の判断として保存'),
        ),
      ],
    ),
  );
}

class _MessageCard extends StatelessWidget {
  const _MessageCard(this.message);

  final ConversationMessage message;

  @override
  Widget build(BuildContext context) => Container(
    margin: const EdgeInsets.only(bottom: 8),
    padding: const EdgeInsets.all(14),
    decoration: BoxDecoration(
      color: message.role == 'user' ? const Color(0xfffbe3dc) : Colors.white,
      borderRadius: BorderRadius.circular(15),
    ),
    child: Text(message.text.isEmpty ? '考えています…' : message.text),
  );
}

class _Composer extends StatelessWidget {
  const _Composer({
    required this.controller,
    required this.enabled,
    required this.onSend,
  });

  final TextEditingController controller;
  final bool enabled;
  final VoidCallback onSend;

  @override
  Widget build(BuildContext context) => SafeArea(
    top: false,
    child: Container(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
      decoration: const BoxDecoration(
        color: Color(0xfff1f5f4),
        border: Border(top: BorderSide(color: Color(0xffd8e2e1))),
      ),
      child: Row(
        children: [
          Expanded(
            child: TextField(
              key: const Key('triage-input'),
              controller: controller,
              enabled: enabled,
              minLines: 1,
              maxLines: 3,
              textInputAction: TextInputAction.send,
              onSubmitted: enabled ? (_) => onSend() : null,
              decoration: const InputDecoration(hintText: '気になっていることを、そのまま書く'),
            ),
          ),
          const SizedBox(width: 8),
          IconButton.filled(
            tooltip: '送信',
            onPressed: enabled ? onSend : null,
            icon: const Icon(Icons.arrow_upward_rounded),
          ),
        ],
      ),
    ),
  );
}

class _SectionLabel extends StatelessWidget {
  const _SectionLabel(this.text);

  final String text;

  @override
  Widget build(BuildContext context) =>
      Text(text, style: Theme.of(context).textTheme.headlineSmall);
}

class _Notice extends StatelessWidget {
  const _Notice({required this.text, this.onClose});

  final String text;
  final VoidCallback? onClose;

  @override
  Widget build(BuildContext context) => Material(
    color: const Color(0xffffe5df),
    borderRadius: BorderRadius.circular(14),
    child: Padding(
      padding: const EdgeInsets.fromLTRB(14, 10, 6, 10),
      child: Row(
        children: [
          const Icon(Icons.info_outline_rounded, size: 20),
          const SizedBox(width: 10),
          Expanded(child: Text(text)),
          if (onClose != null)
            IconButton(
              tooltip: '閉じる',
              onPressed: onClose,
              icon: const Icon(Icons.close_rounded, size: 18),
            ),
        ],
      ),
    ),
  );
}
