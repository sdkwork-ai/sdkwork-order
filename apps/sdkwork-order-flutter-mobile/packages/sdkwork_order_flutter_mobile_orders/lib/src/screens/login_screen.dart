import 'package:flutter/material.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

/// 登录：IAM Dart 家族落地前使用平台签发的访问令牌（手输或 bootstrap 注入）.
class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key, this.session});

  /// Defaults to the global session when omitted.
  final AppSession? session;

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final TextEditingController _token = TextEditingController();
  bool _obscure = true;
  bool _busy = false;

  AppSession get _session => widget.session ?? AppSession.instance;

  @override
  void dispose() {
    _token.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('登录')),
      body: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          Text(
            '移动端 IAM 合约（原生登录/code2session）落地前，'
            '使用平台签发的访问令牌登录。令牌仅保存在内存会话中。',
            style: Theme.of(context).textTheme.bodySmall,
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _token,
            obscureText: _obscure,
            autofocus: true,
            decoration: InputDecoration(
              labelText: '访问令牌',
              border: const OutlineInputBorder(),
              suffixIcon: IconButton(
                icon: Icon(_obscure ? Icons.visibility_off : Icons.visibility),
                onPressed: () => setState(() => _obscure = !_obscure),
              ),
            ),
          ),
          const SizedBox(height: 16),
          FilledButton(
            onPressed: _busy ? null : _signIn,
            child: Text(_busy ? '登录中...' : '登录'),
          ),
          if (_session.isLoggedIn) ...[
            const SizedBox(height: 12),
            OutlinedButton(onPressed: _signOut, child: const Text('清除令牌并退出')),
          ],
        ],
      ),
    );
  }

  void _signIn() {
    final token = _token.text.trim();
    if (token.isEmpty) {
      ScaffoldMessenger.of(context)
          .showSnackBar(const SnackBar(content: Text('请输入访问令牌')));
      return;
    }
    setState(() => _busy = true);
    _session.signIn(token);
    if (mounted) {
      Navigator.of(context).pop();
    }
  }

  void _signOut() {
    _session.signOut();
    if (mounted) {
      setState(() => _token.clear());
    }
  }
}
