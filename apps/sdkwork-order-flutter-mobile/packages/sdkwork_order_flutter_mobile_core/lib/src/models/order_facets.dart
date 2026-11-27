/// Order-centre scene and tag facets.
///
/// Mirrors the backend `OrderScene` / `OrderTag` vocabulary and the TypeScript
/// `SDKWORK_ORDER_SCENES` constant so every surface (PC admin, PC buyer, H5,
/// mini program, Flutter) offers exactly the same filter dimensions. The server
/// rejects an unknown scene, so this list is the complete option set.
library;

/// Canonical commerce scenes, in the order surfaces present them.
const List<String> kOrderScenes = <String>[
  'web',
  'h5',
  'app',
  'mini_program',
  'cashier',
  'membership',
  'recharge',
  'platform',
  'default',
];

/// Maximum tags one order may carry (mirrors `ORDER_TAG_MAX_COUNT`).
const int kOrderTagMaxCount = 20;

/// Maximum length of one canonical tag (mirrors `ORDER_TAG_MAX_LENGTH`).
const int kOrderTagMaxLength = 32;

/// How multiple selected tags combine.
enum OrderTagMatch { any, all }

extension OrderTagMatchWire on OrderTagMatch {
  /// Wire value accepted by the `tag_match` query parameter.
  String get wire => this == OrderTagMatch.all ? 'all' : 'any';
}

/// Returns `true` when [value] is one of [kOrderScenes].
bool isOrderScene(String value) => kOrderScenes.contains(value);

/// Canonicalizes one tag.
///
/// Mirrors the server rule exactly: lowercase, alphanumerics plus `-`
/// (whitespace, `.`, `_`, and `-` all fold into a single `-`), no leading or
/// trailing separator, at most [kOrderTagMaxLength] characters. Returns `null`
/// for input that normalizes to nothing or contains a character the server
/// would reject, so the caller can drop a stray token silently.
String? normalizeOrderTag(String value) {
  final buffer = StringBuffer();
  var pendingSeparator = false;
  for (final rune in value.trim().runes) {
    final character = String.fromCharCode(rune);
    final isWord = RegExp(r'[\p{L}\p{N}]', unicode: true).hasMatch(character);
    if (isWord) {
      if (pendingSeparator && buffer.isNotEmpty) {
        buffer.write('-');
      }
      buffer.write(character.toLowerCase());
      pendingSeparator = false;
      continue;
    }
    if (character == '_' ||
        character == '-' ||
        character == '.' ||
        RegExp(r'\s', unicode: true).hasMatch(character)) {
      pendingSeparator = true;
      continue;
    }
    return null;
  }
  final canonical = buffer.toString();
  if (canonical.isEmpty) {
    return null;
  }
  if (canonical.runes.length > kOrderTagMaxLength) {
    return null;
  }
  return canonical;
}

/// Canonicalizes and deduplicates a tag list, preserving first-seen order.
///
/// Accepts the shapes a filter input produces: a comma/pipe separated string or
/// an already-split list.
List<String> normalizeOrderTags(Iterable<String> values) {
  final tags = <String>[];
  for (final raw in values) {
    for (final part in raw.split(RegExp(r'[,|]'))) {
      final tag = normalizeOrderTag(part);
      if (tag != null && !tags.contains(tag)) {
        tags.add(tag);
      }
    }
  }
  return List<String>.unmodifiable(tags);
}

/// Scene and tag filters applied to the buyer order list.
class OrderListFacets {
  const OrderListFacets({
    this.scene,
    this.tags = const <String>[],
    this.tagMatch = OrderTagMatch.any,
  });

  /// Selected scene, or `null` for "all scenes".
  final String? scene;

  /// Canonical tags; empty for "all tags".
  final List<String> tags;

  /// How [tags] combine.
  final OrderTagMatch tagMatch;

  /// `true` when at least one facet narrows the list.
  bool get isActive => scene != null || tags.isNotEmpty;

  /// Returns a copy with the given fields replaced.
  OrderListFacets copyWith({
    String? scene,
    bool clearScene = false,
    List<String>? tags,
    OrderTagMatch? tagMatch,
  }) =>
      OrderListFacets(
        scene: clearScene ? null : (scene ?? this.scene),
        tags: tags ?? this.tags,
        tagMatch: tagMatch ?? this.tagMatch,
      );

  /// Adds [tag] when it is legal and not already selected.
  OrderListFacets withAddedTag(String tag) {
    final normalized = normalizeOrderTag(tag);
    if (normalized == null || tags.contains(normalized)) {
      return this;
    }
    return copyWith(tags: <String>[...tags, normalized]);
  }

  /// Removes [tag] from the selection.
  OrderListFacets withoutTag(String tag) =>
      copyWith(tags: tags.where((value) => value != tag).toList(growable: false));

  @override
  bool operator ==(Object other) =>
      other is OrderListFacets &&
      other.scene == scene &&
      other.tagMatch == tagMatch &&
      other.tags.length == tags.length &&
      List<String>.from(other.tags).join('\u0000') == tags.join('\u0000');

  @override
  int get hashCode => Object.hash(scene, tagMatch, tags.join('\u0000'));
}
