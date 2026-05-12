import 'package:flutter/foundation.dart';
import 'package:logging/logging.dart';

enum LogLevel {
  debug,
  info,
  warning,
  error,
}

class AppLogger {
  static final AppLogger _instance = AppLogger._internal();
  factory AppLogger() => _instance;
  AppLogger._internal();

  Logger? _logger;
  final _subscribers = <LogSubscriber>[];

  void init({LogLevel minLevel = LogLevel.debug}) {
    _logger = Logger('MusicRoom');

    Logger.root.level = _mapLevel(minLevel);

    Logger.root.onRecord.listen((record) {
      final entry = LogEntry(
        level: _mapToLogLevel(record.level),
        message: record.message,
        loggerName: record.loggerName,
        error: record.error?.toString(),
        stackTrace: record.stackTrace?.toString(),
        time: record.time,
      );

      for (final sub in _subscribers) {
        sub(entry);
      }

      if (kDebugMode) {
        _printToConsole(entry);
      }
    });
  }

  Level _mapLevel(LogLevel level) {
    switch (level) {
      case LogLevel.debug:
        return Level.FINE;
      case LogLevel.info:
        return Level.INFO;
      case LogLevel.warning:
        return Level.WARNING;
      case LogLevel.error:
        return Level.SEVERE;
    }
  }

  LogLevel _mapToLogLevel(Level level) {
    if (level == Level.FINE) return LogLevel.debug;
    if (level == Level.INFO) return LogLevel.info;
    if (level == Level.WARNING) return LogLevel.warning;
    return LogLevel.error;
  }

  void _printToConsole(LogEntry entry) {
    final prefix = switch (entry.level) {
      LogLevel.debug => '[D]',
      LogLevel.info => '[I]',
      LogLevel.warning => '[W]',
      LogLevel.error => '[E]',
    };

    final errorInfo = entry.error != null ? ' | ${entry.error}' : '';
    debugPrint('$prefix ${entry.loggerName}: ${entry.message}$errorInfo');
  }

  void subscribe(LogSubscriber callback) {
    _subscribers.add(callback);
  }

  void unsubscribe(LogSubscriber callback) {
    _subscribers.remove(callback);
  }

  void debug(String message, {String? source}) {
    _logger?.fine(message);
  }

  void info(String message, {String? source}) {
    _logger?.info(message);
  }

  void warning(String message, {String? source}) {
    _logger?.warning(message);
  }

  void error(String message, {Object? error, StackTrace? stackTrace}) {
    _logger?.severe(message, error, stackTrace);
  }
}

typedef LogSubscriber = void Function(LogEntry entry);

class LogEntry {
  final LogLevel level;
  final String message;
  final String loggerName;
  final String? error;
  final String? stackTrace;
  final DateTime time;

  LogEntry({
    required this.level,
    required this.message,
    required this.loggerName,
    this.error,
    this.stackTrace,
    required this.time,
  });
}

final logger = AppLogger();
