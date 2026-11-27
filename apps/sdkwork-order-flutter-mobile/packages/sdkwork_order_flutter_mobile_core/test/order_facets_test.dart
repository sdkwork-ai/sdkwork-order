import 'package:flutter_test/flutter_test.dart';
import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

void main() {
  group('order scene vocabulary', () {
    test('lists exactly the scenes the backend accepts', () {
      expect(kOrderScenes, <String>[
        'web',
        'h5',
        'app',
        'mini_program',
        'cashier',
        'membership',
        'recharge',
        'platform',
        'default',
      ]);
    });

    test('recognizes only canonical scenes', () {
      for (final scene in kOrderScenes) {
        expect(isOrderScene(scene), isTrue, reason: scene);
      }
      expect(isOrderScene('pc'), isFalse);
      expect(isOrderScene('telepathy'), isFalse);
    });
  });

  group('order tag canonicalization', () {
    test('lowercases and folds separators', () {
      expect(normalizeOrderTag('VIP'), 'vip');
      expect(normalizeOrderTag('  vip  '), 'vip');
      expect(normalizeOrderTag('summer sale'), 'summer-sale');
      expect(normalizeOrderTag('Risk__Review'), 'risk-review');
      expect(normalizeOrderTag('a--b'), 'a-b');
      expect(normalizeOrderTag('v2.0'), 'v2-0');
    });

    test('never starts or ends with a separator', () {
      expect(normalizeOrderTag('-lead'), 'lead');
      expect(normalizeOrderTag('trail-'), 'trail');
      expect(normalizeOrderTag(' -- both -- '), 'both');
    });

    test('keeps CJK tags', () {
      expect(normalizeOrderTag('高价值'), '高价值');
    });

    test('is idempotent', () {
      for (final input in <String>['VIP', 'summer sale', 'a--b', '-lead', '高价值']) {
        final once = normalizeOrderTag(input);
        expect(once, isNotNull, reason: input);
        expect(normalizeOrderTag(once!), once, reason: input);
      }
    });

    test('rejects blank input', () {
      for (final input in <String>['', '   ', '---', '___', ' - ']) {
        expect(normalizeOrderTag(input), isNull, reason: input);
      }
    });

    test('rejects characters the server rejects', () {
      expect(normalizeOrderTag('vip!'), isNull);
      expect(normalizeOrderTag('a/b'), isNull);
      expect(normalizeOrderTag('vip" OR 1=1'), isNull);
    });

    test('enforces the length ceiling', () {
      expect(normalizeOrderTag('x' * kOrderTagMaxLength), isNotNull);
      expect(normalizeOrderTag('x' * (kOrderTagMaxLength + 1)), isNull);
    });
  });

  group('order tag lists', () {
    test('splits and deduplicates while preserving order', () {
      expect(normalizeOrderTags(<String>['VIP,vip', 'risk|VIP']), <String>['vip', 'risk']);
    });

    test('drops blank and invalid tokens', () {
      expect(normalizeOrderTags(<String>['vip', ' ', 'bad!', 'risk']), <String>['vip', 'risk']);
    });
  });

  group('OrderListFacets', () {
    test('is inactive until a scene or tag is selected', () {
      expect(const OrderListFacets().isActive, isFalse);
      expect(const OrderListFacets(scene: 'h5').isActive, isTrue);
      expect(const OrderListFacets(tags: <String>['vip']).isActive, isTrue);
    });

    test('adds a normalized tag and ignores duplicates', () {
      const base = OrderListFacets();
      final once = base.withAddedTag('VIP');
      expect(once.tags, <String>['vip']);
      expect(once.withAddedTag('vip').tags, <String>['vip']);
    });

    test('ignores an illegal tag instead of corrupting the filter', () {
      expect(const OrderListFacets().withAddedTag('bad!').tags, isEmpty);
      expect(const OrderListFacets().withAddedTag('   ').tags, isEmpty);
    });

    test('removes a tag', () {
      const base = OrderListFacets(tags: <String>['vip', 'risk']);
      expect(base.withoutTag('vip').tags, <String>['risk']);
    });

    test('clears the scene explicitly', () {
      const base = OrderListFacets(scene: 'h5');
      expect(base.copyWith(clearScene: true).scene, isNull);
      // Omitting both keeps the current scene.
      expect(base.copyWith().scene, 'h5');
    });

    test('maps the match mode onto the wire value', () {
      expect(OrderTagMatch.any.wire, 'any');
      expect(OrderTagMatch.all.wire, 'all');
    });
  });
}
