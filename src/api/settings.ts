import type {
  DocumentTemplateDto,
  FeatureFlagsDto,
  GenericImportHeaderRequest,
  GenericImportHeadersDto,
  GenericImportPreviewDto,
  GenericImportReportRequest,
  GenericImportRequest,
  GenericImportResultDto,
  ImportMappingDto,
  IndustryTemplateDto,
  MerchantProfileDto,
  SettingDto,
  SetupStatusDto,
  TermSettingsDto
} from '../shared/types';
import { callCommand } from './tauri';

export const settingsApi = {
  settings: () => callCommand<SettingDto[]>('list_settings'),
  saveSettings: (payload: Record<string, unknown>) => callCommand<boolean>('save_settings', { payload }),
  setupStatus: () => callCommand<SetupStatusDto>('get_setup_status'),
  completeSetup: (request: Record<string, unknown>) => callCommand<boolean>('complete_setup', { request }),
  merchantProfile: () => callCommand<MerchantProfileDto>('get_merchant_profile'),
  saveMerchantProfile: (profile: MerchantProfileDto) => callCommand<boolean>('save_merchant_profile', { profile }),
  termSettings: () => callCommand<TermSettingsDto>('get_term_settings'),
  saveTermSettings: (terms: TermSettingsDto) => callCommand<boolean>('save_term_settings', { terms }),
  featureFlags: () => callCommand<FeatureFlagsDto>('get_feature_flags'),
  saveFeatureFlags: (flags: FeatureFlagsDto) => callCommand<boolean>('save_feature_flags', { flags }),
  industryTemplates: () => callCommand<IndustryTemplateDto[]>('list_industry_templates'),
  applyIndustryTemplate: (request: Record<string, unknown>) =>
    callCommand<IndustryTemplateDto>('apply_industry_template', { request }),
  documentTemplates: () => callCommand<DocumentTemplateDto[]>('list_document_templates'),
  applyDocumentTemplate: (templateId: string) => callCommand<boolean>('apply_document_template', { templateId }),
  previewGenericImportHeaders: (request: GenericImportHeaderRequest) =>
    callCommand<GenericImportHeadersDto>('preview_generic_import_headers', { request }),
  previewGenericImport: (request: GenericImportRequest) =>
    callCommand<GenericImportPreviewDto>('preview_generic_import', { request }),
  confirmGenericImport: (request: GenericImportRequest) =>
    callCommand<GenericImportResultDto>('confirm_generic_import', { request }),
  exportGenericImportReport: (request: GenericImportReportRequest) =>
    callCommand<string>('export_generic_import_report', { request }),
  downloadImportTemplate: (importType: GenericImportRequest['importType']) =>
    callCommand<string>('download_import_template', { importType }),
  saveImportMapping: (mapping: ImportMappingDto) => callCommand<boolean>('save_import_mapping', { mapping }),
  importMappings: (importType?: string) => callCommand<ImportMappingDto[]>('list_import_mappings', { importType })
};
