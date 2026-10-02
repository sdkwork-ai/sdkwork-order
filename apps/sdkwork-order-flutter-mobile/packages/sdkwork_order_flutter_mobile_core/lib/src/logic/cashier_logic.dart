/// Pure cashier state-machine helpers shared by the cashier screen and its
/// unit tests. Mirrors the mobile-react `CashierLogic` contract so the
/// Flutter cashier stays aligned with backend payment/order semantics.
library;

/// Normal poll cadence: 3s.
const int cashierPollIntervalMs = 3000;

/// Poll backoff multiplier applied while the network is unstable.
const int cashierPollBackoffMultiplier = 2;

/// Consecutive poll failures that switch the cashier onto backoff.
const int cashierPollBackoffAfterFailures = 3;

/// Backend payment/order TTL applied at cashier creation (15 minutes).
const int cashierTtlSeconds = 15 * 60;

/// Terminal cashier phases (plus `pending` while waiting for the provider).
enum CashierPhase { pending, paid, cancelled, expired, failed }

/// Maps a `payment_success` response (`{paid, status, statusName}`) plus the
/// order status onto the cashier phase.
CashierPhase resolveCashierPhase({
  required bool paid,
  required String orderStatus,
}) {
  if (paid) {
    return CashierPhase.paid;
  }
  final normalized = orderStatus.trim().toLowerCase();
  if (normalized == 'cancelled' || normalized == 'canceled' || normalized == 'closed') {
    return CashierPhase.cancelled;
  }
  if (normalized == 'expired' || normalized == 'timeout') {
    return CashierPhase.expired;
  }
  if (normalized == 'failed') {
    return CashierPhase.failed;
  }
  if (normalized == 'paid' ||
      normalized == 'fulfilled' ||
      normalized == 'completed' ||
      normalized == 'shipped' ||
      normalized == 'delivered' ||
      normalized == 'refunding' ||
      normalized == 'refunded') {
    return CashierPhase.paid;
  }
  return CashierPhase.pending;
}

/// Next poll delay in ms: normal cadence, doubled once the consecutive
/// failure count reaches [cashierPollBackoffAfterFailures].
int nextPollDelayMs({required int consecutiveFailures}) {
  if (consecutiveFailures >= cashierPollBackoffAfterFailures) {
    return cashierPollIntervalMs * cashierPollBackoffMultiplier;
  }
  return cashierPollIntervalMs;
}

/// Seconds remaining until the cashier closes, floored at zero. The deadline
/// is the earlier of the order `expireTime` and
/// paymentCreatedAt + [cashierTtlSeconds].
int computeCashierRemainingSeconds(
  String? expireTimeIso,
  int paymentCreatedAtMs,
  int nowMs,
) {
  final candidates = <int>[paymentCreatedAtMs + cashierTtlSeconds * 1000];
  final expireTime = expireTimeIso?.trim() ?? '';
  if (expireTime.isNotEmpty) {
    final parsed = DateTime.tryParse(expireTime);
    if (parsed != null) {
      candidates.add(parsed.millisecondsSinceEpoch);
    }
  }
  var deadline = candidates.first;
  for (final candidate in candidates.skip(1)) {
    if (candidate < deadline) {
      deadline = candidate;
    }
  }
  final remaining = ((deadline - nowMs) / 1000).floor();
  return remaining > 0 ? remaining : 0;
}

/// Formats remaining seconds as `mm:ss`.
String formatCashierCountdown(int remainingSeconds) {
  final seconds = remainingSeconds > 0 ? remainingSeconds : 0;
  final minutes = seconds ~/ 60;
  final rest = seconds % 60;
  return '${minutes.toString().padLeft(2, '0')}:${rest.toString().padLeft(2, '0')}';
}

/// True when the phase allows re-entering the cashier (retry payment).
bool isCashierRetryablePhase(CashierPhase phase) =>
    phase == CashierPhase.failed || phase == CashierPhase.expired;
