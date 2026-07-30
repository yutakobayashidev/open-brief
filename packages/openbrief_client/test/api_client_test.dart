import 'package:openbrief_client/openbrief_client.dart';
import 'package:test/test.dart';

void main() {
  test('requires HTTPS except for loopback development', () {
    expect(
      OpenBriefApi.normalizeBaseUrl('https://openbrief.example/'),
      'https://openbrief.example',
    );
    expect(
      OpenBriefApi.normalizeBaseUrl('http://127.0.0.1:43117'),
      'http://127.0.0.1:43117',
    );
    expect(
      () => OpenBriefApi.normalizeBaseUrl('http://openbrief.example'),
      throwsFormatException,
    );
  });

  test('parses a finite snapshot', () {
    final snapshot = RemoteSnapshot.fromJson({
      'brief': {
        'protect': [
          {
            'id': 'mail-1',
            'title': 'Reply',
            'reason': 'Waiting on you',
            'source': 'gmail',
            'observedAt': '2026-07-31T09:00:00+09:00',
          },
        ],
        'explore': <dynamic>[],
        'coverage': <dynamic>[],
        'generatedAt': '2026-07-31T09:00:00+09:00',
      },
      'returnThread': null,
      'pendingProposal': null,
      'agent': {
        'availability': {'status': 'available'},
        'authentication': {'status': 'authenticated'},
        'process': {'status': 'ready'},
        'runtime': null,
      },
      'nextSequence': 4,
    });

    expect(snapshot.brief.length, 1);
    expect(snapshot.brief.protect.single.id, 'mail-1');
    expect(snapshot.agent.ready, isTrue);
    expect(snapshot.nextSequence, 4);
  });
}
