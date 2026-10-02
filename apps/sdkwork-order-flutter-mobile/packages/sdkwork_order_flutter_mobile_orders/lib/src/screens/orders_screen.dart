import 'package:flutter/material.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

import '../l10n/strings.dart';
import '../widgets/common_views.dart';

/// 订单中心：状态页签 + 服务端分页（一页一页请求，无全量下载）.
class OrdersScreen extends StatefulWidget {
  const OrdersScreen({super.key, required this.orderService, this.initialTab = 'all'});

  final OrderService orderService;
  final String initialTab;

  @override
  State<OrdersScreen> createState() => _OrdersScreenState();
}

class _OrdersScreenState extends State<OrdersScreen> {
  static const int _pageSize = 20;

  late String _activeTab = widget.initialTab;
  final List<Order> _orders = <Order>[];
  final ScrollController _scrollController = ScrollController();
  bool _hasMore = false;
  int _page = 1;
  bool _loading = true;
  bool _loadingMore = false;
  String _error = '';

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_onScroll);
    _loadFirstPage();
  }

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  void _onScroll() {
    if (!_hasMore ||
        _loadingMore ||
        !_scrollController.hasClients ||
        _scrollController.position.extentAfter > 240) {
      return;
    }
    _loadNextPage();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        SizedBox(
          height: 48,
          child: ListView(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.symmetric(horizontal: 8),
            children: orderTabs
                .map(
                  (tab) => Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 4),
                    child: ChoiceChip(
                      label: Text(tab.value),
                      selected: _activeTab == tab.key,
                      onSelected: (_) {
                        setState(() => _activeTab = tab.key);
                        _loadFirstPage();
                      },
                    ),
                  ),
                )
                .toList(),
          ),
        ),
        Expanded(
          child: _loading
              ? const SdkworkLoadingView(label: '加载订单...')
              : _error.isNotEmpty && _orders.isEmpty
                  ? SdkworkErrorView(message: _error, onRetry: _loadFirstPage)
                  : _orders.isEmpty
                      ? const SdkworkEmptyView(message: '暂无订单')
                      : RefreshIndicator(
                          onRefresh: _loadFirstPage,
                          child: ListView.builder(
                            controller: _scrollController,
                            padding: const EdgeInsets.all(12),
                            itemCount: _orders.length + (_hasMore ? 1 : 0),
                            itemBuilder: (context, index) {
                              if (index >= _orders.length) {
                                return const Padding(
                                  padding: EdgeInsets.all(16),
                                  child: Center(
                                    child: SizedBox(
                                      width: 24,
                                      height: 24,
                                      child: CircularProgressIndicator(
                                        strokeWidth: 2,
                                      ),
                                    ),
                                  ),
                                );
                              }
                              return _buildOrderCard(_orders[index]);
                            },
                          ),
                        ),
        ),
      ],
    );
  }

  Widget _buildOrderCard(Order order) {
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    order.subject,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                ),
                Text(
                  statusLabel(order.status, statusName: order.statusName),
                  style: TextStyle(
                    color: statusColor(context, order.status),
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  formatTime(order.createdAt),
                  style: Theme.of(context).textTheme.labelSmall,
                ),
                Text(
                  formatAmount(order.totalAmount, currencyCode: order.currencyCode),
                  style: TextStyle(
                    color: Theme.of(context).colorScheme.primary,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: () =>
                      Navigator.of(context).pushNamed('/order-detail', arguments: order.id),
                  child: const Text('订单详情'),
                ),
                if (order.isPendingPayment) ...[
                  const SizedBox(width: 8),
                  FilledButton(
                    onPressed: () =>
                        Navigator.of(context).pushNamed('/cashier', arguments: order.id),
                    child: const Text('去支付'),
                  ),
                  const SizedBox(width: 8),
                  OutlinedButton(
                    onPressed: () => _cancel(order),
                    child: const Text('取消'),
                  ),
                ],
              ],
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _loadFirstPage() async {
    setState(() {
      _loading = _orders.isEmpty;
      _loadingMore = false;
      _error = '';
    });
    try {
      final page = await widget.orderService.listOrders(
        status: _activeTab == 'all' ? null : _activeTab,
        page: 1,
        pageSize: _pageSize,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _orders
          ..clear()
          ..addAll(page.items);
        _page = 1;
        _hasMore = page.pageInfo.hasMore;
        _loading = false;
      });
    } catch (cause) {
      if (!mounted) {
        return;
      }
      setState(() {
        _loading = false;
        _error = '$cause';
      });
    }
  }

  Future<void> _loadNextPage() async {
    setState(() => _loadingMore = true);
    try {
      final next = await widget.orderService.listOrders(
        status: _activeTab == 'all' ? null : _activeTab,
        page: _page + 1,
        pageSize: _pageSize,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _orders.addAll(next.items);
        _page += 1;
        _hasMore = next.pageInfo.hasMore;
        _loadingMore = false;
      });
    } catch (cause) {
      if (!mounted) {
        return;
      }
      setState(() {
        _loadingMore = false;
        _error = '$cause';
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('加载更多失败：$cause')),
      );
    }
  }

  Future<void> _cancel(Order order) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('取消订单'),
        content: const Text('确定要取消该订单吗？'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: const Text('再想想'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: const Text('取消订单'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    try {
      await widget.orderService.cancelOrder(order.id);
      await _loadFirstPage();
    } catch (cause) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text('$cause')));
    }
  }
}
