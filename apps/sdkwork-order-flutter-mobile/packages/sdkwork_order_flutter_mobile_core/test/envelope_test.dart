import 'package:flutter_test/flutter_test.dart';

import 'package:sdkwork_order_flutter_mobile_core/src/logic/envelope.dart';

void main() {
  group('resourceItem', () {
    test('unwraps data.item', () {
      final data = <String, dynamic>{
        'item': <String, dynamic>{'orderId': '42', 'status': 'pending_payment'},
      };
      expect(resourceItem(data)['orderId'], '42');
    });

    test('returns an empty map when the item key exists but is not a map', () {
      expect(resourceItem(<String, dynamic>{'item': 'broken'}), isEmpty);
    });

    test('tolerates already-unwrapped payloads', () {
      final data = <String, dynamic>{'orderId': '42'};
      expect(resourceItem(data)['orderId'], '42');
    });
  });

  group('listItems + readPageInfo', () {
    test('unwraps data.items and pageInfo', () {
      final data = <String, dynamic>{
        'items': <Object?>[
          <String, dynamic>{'orderId': '1'},
          <String, dynamic>{'orderId': '2'},
          'junk',
        ],
        'pageInfo': <String, dynamic>{
          'mode': 'offset',
          'page': 2,
          'pageSize': 20,
          'totalItems': '41',
          'hasMore': true,
        },
      };
      final items = listItems(data);
      expect(items.length, 2);
      expect(items.first['orderId'], '1');

      final pageInfo = readPageInfo(data);
      expect(pageInfo.mode, 'offset');
      expect(pageInfo.page, 2);
      expect(pageInfo.pageSize, 20);
      expect(pageInfo.totalItems, '41');
      expect(pageInfo.hasMore, isTrue);
    });

    test('applies offset defaults for a missing pageInfo', () {
      final pageInfo = readPageInfo(<String, dynamic>{});
      expect(pageInfo.mode, 'offset');
      expect(pageInfo.page, 1);
      expect(pageInfo.pageSize, 20);
      expect(pageInfo.totalItems, '0');
      expect(pageInfo.hasMore, isFalse);
    });
  });

  group('typed accessors', () {
    test('asString picks the first present key', () {
      expect(
        asString(<String, dynamic>{'b': 'x'}, const ['a', 'b']),
        'x',
      );
      expect(asString(<String, dynamic>{}, const ['a'], fallback: 'z'), 'z');
      expect(asString(<String, dynamic>{'a': ''}, const ['a']), '');
    });

    test('asInt parses num and string forms', () {
      expect(asInt(<String, dynamic>{'a': 3}, const ['a']), 3);
      expect(asInt(<String, dynamic>{'a': '12'}, const ['a']), 12);
      expect(asInt(<String, dynamic>{'a': 'x'}, const ['a'], fallback: 7), 7);
    });

    test('asBool only accepts booleans', () {
      expect(asBool(<String, dynamic>{'a': true}, const ['a']), isTrue);
      expect(asBool(<String, dynamic>{'a': 'true'}, const ['a']), isFalse);
    });
  });
}
