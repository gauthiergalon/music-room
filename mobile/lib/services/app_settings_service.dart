import 'package:shared_preferences/shared_preferences.dart';

class AppSettingsService {
  static const String _batteryOptimizationDismissedKey = 'battery_optimization_dismissed';

  static AppSettingsService? _instance;
  static SharedPreferences? _prefs;

  AppSettingsService._();

  static Future<AppSettingsService> getInstance() async {
    _instance ??= AppSettingsService._();
    _prefs ??= await SharedPreferences.getInstance();
    return _instance!;
  }

  bool get batteryOptimizationDismissed {
    return _prefs?.getBool(_batteryOptimizationDismissedKey) ?? false;
  }

  Future<void> setBatteryOptimizationDismissed(bool dismissed) async {
    await _prefs?.setBool(_batteryOptimizationDismissedKey, dismissed);
  }
}