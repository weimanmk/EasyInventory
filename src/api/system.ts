import type {
  AppStatusDto,
  AuditLogDto,
  BackupDto,
  DataSelfCheckDto,
  DiagnosticPackageDto,
  DiagnosticSummaryDto,
  ImportResult,
  RestoreBackupResultDto
} from '../shared/types';
import { callCommand } from './tauri';

export const systemApi = {
  status: () => callCommand<AppStatusDto>('get_app_status'),
  openExportsFolder: () => callCommand<string>('open_exports_folder'),
  openLogsFolder: () => callCommand<string>('open_logs_folder'),
  runDataSelfCheck: () => callCommand<DataSelfCheckDto>('run_data_self_check'),
  exportDataSelfCheck: () => callCommand<string>('export_data_self_check'),
  diagnosticSummary: () => callCommand<DiagnosticSummaryDto>('get_diagnostic_summary'),
  exportDiagnosticPackage: () => callCommand<DiagnosticPackageDto>('export_diagnostic_package'),
  importExcel: (filePath: string) => callCommand<ImportResult>('import_excel', { filePath }),
  importStatus: () => callCommand<ImportResult | null>('get_import_status'),
  backup: () => callCommand<string>('create_backup'),
  backups: () => callCommand<BackupDto[]>('list_backups'),
  openBackupFolder: () => callCommand<string>('open_backup_folder'),
  restoreBackup: (backupId: number) => callCommand<RestoreBackupResultDto>('restore_backup', { backupId }),
  auditLogs: (filter?: Record<string, unknown>) => callCommand<AuditLogDto[]>('list_audit_logs', { filter }),
  printers: () => callCommand<string[]>('list_printers')
};
