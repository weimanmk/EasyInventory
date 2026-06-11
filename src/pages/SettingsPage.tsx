import {
  Alert,
  App,
  Button,
  Card,
  Collapse,
  Descriptions,
  Form,
  Input,
  List,
  Select,
  Space,
  Statistic,
  Table,
  Tag,
  Typography
} from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { api } from '../api/inventory';
import type {
  AppStatusDto,
  AuditLogDto,
  BackupDto,
  DataSelfCheckDto,
  DiagnosticSummaryDto,
  DocumentTemplateDto,
  FeatureFlagsDto,
  GenericImportHeadersDto,
  GenericImportPreviewDto,
  GenericImportRequest,
  GenericImportResultDto,
  ImportMappingDto,
  ImportResult,
  IndustryTemplateDto,
  MerchantProfileDto,
  SettingDto,
  TermSettingsDto
} from '../shared/types';
import { defaultFeatures, defaultMerchant, defaultTerms, useAppStore } from '../store/appStore';
import { AuditLogCard } from './settings/AuditLogCard';
import { BackupRestoreCard } from './settings/BackupRestoreCard';
import { DiagnosticsCard } from './settings/DiagnosticsCard';
import { DocumentTemplateSettingsCard } from './settings/DocumentTemplateSettingsCard';
import { IndustryFeatureCard } from './settings/IndustryFeatureCard';
import { LocalPathsCard } from './settings/LocalPathsCard';
import { MerchantProfileCard } from './settings/MerchantProfileCard';
import { SetupGuideCard } from './settings/SetupGuideCard';
import { TermSettingsCard } from './settings/TermSettingsCard';

type GenericImportForm = {
  importType: GenericImportRequest['importType'];
  filePath: string;
  duplicateStrategy: GenericImportRequest['duplicateStrategy'];
  mappingName?: string;
  fieldMappingText?: string;
};

const genericImportTypeOptions = [
  { value: 'products', label: '通用商品导入' },
  { value: 'customers', label: '通用客户导入' },
  { value: 'initial_stock', label: '通用期初库存导入' }
];

export default function SettingsPage() {
  const { message, modal } = App.useApp();
  const navigate = useNavigate();
  const {
    status,
    merchant,
    terms,
    features,
    setStatus,
    setProducts,
    setCustomers,
    setMerchant,
    setTerms,
    setFeatures,
    setSetupStatus
  } = useAppStore();
  const [settingsForm] = Form.useForm();
  const [merchantForm] = Form.useForm<MerchantProfileDto>();
  const [termsForm] = Form.useForm<TermSettingsDto>();
  const [featuresForm] = Form.useForm<FeatureFlagsDto>();
  const [genericImportForm] = Form.useForm<GenericImportForm>();
  const [appStatus, setAppStatus] = useState<AppStatusDto | undefined>(status);
  const [legacyExcelPath, setLegacyExcelPath] = useState('');
  const [legacyImportResult, setLegacyImportResult] = useState<ImportResult | null>();
  const [genericHeaders, setGenericHeaders] = useState<GenericImportHeadersDto | null>(null);
  const [visualFieldMapping, setVisualFieldMapping] = useState<Record<string, string>>({});
  const [genericPreview, setGenericPreview] = useState<GenericImportPreviewDto | null>(null);
  const [genericResult, setGenericResult] = useState<GenericImportResultDto | null>(null);
  const [backups, setBackups] = useState<BackupDto[]>([]);
  const [auditLogs, setAuditLogs] = useState<AuditLogDto[]>([]);
  const [printers, setPrinters] = useState<string[]>([]);
  const [industryTemplates, setIndustryTemplates] = useState<IndustryTemplateDto[]>([]);
  const [documentTemplates, setDocumentTemplates] = useState<DocumentTemplateDto[]>([]);
  const [importMappings, setImportMappings] = useState<ImportMappingDto[]>([]);
  const [selfCheck, setSelfCheck] = useState<DataSelfCheckDto>();
  const [diagnostic, setDiagnostic] = useState<DiagnosticSummaryDto>();
  const templateValues = Form.useWatch([], settingsForm) ?? {};
  const genericImportType = Form.useWatch('importType', genericImportForm);
  const currentMapping = useMemo(
    () => importMappings.find((item) => item.importType === genericImportType),
    [genericImportType, importMappings]
  );
  const missingRequiredMapping = useMemo(
    () =>
      genericHeaders?.fields
        .filter((field) => field.required && !visualFieldMapping[field.name])
        .map((field) => field.name) ?? [],
    [genericHeaders, visualFieldMapping]
  );

  async function refresh() {
    const [
      nextStatus,
      nextBackups,
      settings,
      nextPrinters,
      nextAuditLogs,
      nextDiagnostic,
      nextMerchant,
      nextTerms,
      nextFeatures,
      nextIndustryTemplates,
      nextDocumentTemplates,
      nextImportMappings,
      nextSetupStatus
    ] = await Promise.all([
      api.status(),
      api.backups(),
      api.settings(),
      api.printers().catch(() => [] as string[]),
      api.auditLogs({}),
      api.diagnosticSummary().catch(() => undefined),
      api.merchantProfile().catch(() => defaultMerchant),
      api.termSettings().catch(() => defaultTerms),
      api.featureFlags().catch(() => defaultFeatures),
      api.industryTemplates(),
      api.documentTemplates(),
      api.importMappings(),
      api.setupStatus()
    ]);
    setAppStatus(nextStatus);
    setStatus(nextStatus);
    setBackups(nextBackups);
    setAuditLogs(nextAuditLogs);
    setPrinters(nextPrinters);
    setDiagnostic(nextDiagnostic);
    setMerchant(nextMerchant);
    setTerms(nextTerms);
    setFeatures(nextFeatures);
    setIndustryTemplates(nextIndustryTemplates);
    setDocumentTemplates(nextDocumentTemplates);
    setImportMappings(nextImportMappings);
    setSetupStatus(nextSetupStatus);
    merchantForm.setFieldsValue(nextMerchant);
    termsForm.setFieldsValue(nextTerms);
    featuresForm.setFieldsValue(nextFeatures);
    settingsForm.setFieldsValue(settingsToForm(settings, nextMerchant.name));
    genericImportForm.setFieldsValue({
      importType: 'products',
      duplicateStrategy: 'skip'
    });
  }

  async function refreshMasterData() {
    const [nextProducts, nextCustomers] = await Promise.all([
      api.products({ isActive: true }),
      api.customers({ isActive: true })
    ]);
    setProducts(nextProducts);
    setCustomers(nextCustomers);
  }

  async function saveMerchant() {
    try {
      const values = await merchantForm.validateFields();
      await api.saveMerchantProfile(values);
      const nextMerchant = await api.merchantProfile();
      setMerchant(nextMerchant);
      settingsForm.setFieldValue('templateStoreName', nextMerchant.name || '我的商行');
      message.success('商户信息已保存');
    } catch (error) {
      message.error(error instanceof Error ? error.message : '商户信息保存失败');
    }
  }

  async function saveTerms() {
    try {
      const values = await termsForm.validateFields();
      await api.saveTermSettings(values);
      setTerms(await api.termSettings());
      await refreshMasterData();
      message.success('术语配置已保存');
    } catch (error) {
      message.error(error instanceof Error ? error.message : '术语配置保存失败');
    }
  }

  async function saveFeatures() {
    try {
      const values = await featuresForm.validateFields();
      await api.saveFeatureFlags(values);
      setFeatures(await api.featureFlags());
      message.success('功能开关已保存');
    } catch (error) {
      message.error(error instanceof Error ? error.message : '功能开关保存失败');
    }
  }

  function applyIndustryTemplate(template: IndustryTemplateDto) {
    modal.confirm({
      title: `应用行业模板：${template.name}`,
      content: '行业模板会覆盖术语配置和功能开关，但不会删除任何历史数据。',
      okText: '应用',
      onOk: async () => {
        try {
          await api.applyIndustryTemplate({
            templateId: template.id,
            overwriteTerms: true,
            overwriteFeatures: true
          });
          await refresh();
          message.success('行业模板已应用');
        } catch (error) {
          message.error(error instanceof Error ? error.message : '行业模板应用失败');
        }
      }
    });
  }

  async function applyDocumentTemplate(templateId: string) {
    try {
      await api.applyDocumentTemplate(templateId);
      settingsForm.setFieldValue('defaultPrintTemplate', templateId);
      message.success('单据模板已切换');
      await refresh();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '单据模板切换失败');
    }
  }

  async function importLegacyExcel() {
    if (!legacyExcelPath.trim()) {
      message.warning('请输入历史 Excel 文件路径');
      return;
    }
    try {
      const result = await api.importExcel(legacyExcelPath.trim());
      setLegacyImportResult(result);
      await refreshMasterData();
      message.success(`兼容导入完成：商品 ${result.productCount}，客户 ${result.customerCount}，流水 ${result.movementCount}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '兼容导入失败');
    }
  }

  function genericRequest(values: GenericImportForm): GenericImportRequest {
    const fieldMapping = mergeFieldMapping(visualFieldMapping, values.fieldMappingText);
    return {
      importType: values.importType,
      filePath: values.filePath.trim(),
      duplicateStrategy: values.duplicateStrategy ?? 'skip',
      fieldMapping
    };
  }

  async function previewGenericImportHeaders() {
    try {
      const values = await genericImportForm.validateFields(['importType', 'filePath']);
      const result = await api.previewGenericImportHeaders({
        importType: values.importType,
        filePath: values.filePath.trim()
      });
      setGenericHeaders(result);
      setVisualFieldMapping(result.suggestedMapping);
      setGenericPreview(null);
      setGenericResult(null);
      message.success(`已读取工作表 ${result.sheetName}，识别到 ${result.headers.length} 个表头`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '读取 Excel 表头失败');
    }
  }

  async function previewGenericImport() {
    try {
      const values = await genericImportForm.validateFields();
      if (missingRequiredMapping.length > 0) {
        message.warning(`请先映射必填字段：${missingRequiredMapping.join('、')}`);
        return;
      }
      const preview = await api.previewGenericImport(genericRequest(values));
      setGenericPreview(preview);
      setGenericResult(null);
      message.success(`预览完成：有效 ${preview.validCount} 行，异常 ${preview.errorCount} 行`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '通用导入预览失败');
    }
  }

  async function confirmGenericImport() {
    try {
      const values = await genericImportForm.validateFields();
      const result = await api.confirmGenericImport(genericRequest(values));
      setGenericResult(result);
      await refreshMasterData();
      message.success(`导入完成：成功 ${result.importedCount} 行，异常 ${result.errorCount} 行`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '通用导入失败');
    }
  }

  async function exportGenericImportReport() {
    const rows = genericResult?.rows ?? genericPreview?.rows ?? [];
    if (rows.length === 0) {
      message.warning('请先预览或导入数据');
      return;
    }
    try {
      const values = genericImportForm.getFieldsValue();
      const path = await api.exportGenericImportReport({
        title: `${genericImportTypeText(values.importType)}报告`,
        rows
      });
      message.success(`导入报告已导出：${path}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导入报告导出失败');
    }
  }

  async function downloadImportTemplate() {
    try {
      const importType = genericImportForm.getFieldValue('importType') ?? 'products';
      const path = await api.downloadImportTemplate(importType);
      message.success(`导入模板已导出：${path}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导入模板下载失败');
    }
  }

  async function saveCurrentMapping() {
    try {
      const values = await genericImportForm.validateFields(['importType', 'mappingName', 'fieldMappingText']);
      const name = values.mappingName?.trim();
      if (!name) {
        message.warning('请输入映射方案名称');
        return;
      }
      const fieldMapping = mergeFieldMapping(visualFieldMapping, values.fieldMappingText) ?? {};
      if (Object.keys(fieldMapping).length === 0) {
        message.warning('请先读取表头并配置字段映射');
        return;
      }
      await api.saveImportMapping({
        name,
        importType: values.importType,
        fieldMapping
      });
      setImportMappings(await api.importMappings());
      message.success('映射方案已保存');
    } catch (error) {
      message.error(error instanceof Error ? error.message : '映射方案保存失败');
    }
  }

  function applySavedMapping() {
    if (!currentMapping) {
      message.warning('当前导入类型没有可用的映射方案');
      return;
    }
    setVisualFieldMapping(currentMapping.fieldMapping);
    message.success(`已套用映射方案：${currentMapping.name}`);
  }

  function reopenSetup() {
    modal.confirm({
      title: '重新打开初始化向导？',
      content: '初始化向导只会重新配置商户信息、行业模板、术语、功能开关和默认单据设置，不会清空已有商品、客户、订单或库存流水。',
      okText: '打开向导',
      onOk: () => navigate('/setup')
    });
  }

  async function backup() {
    try {
      const path = await api.backup();
      message.success(`备份完成：${path}`);
      await refresh();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '备份失败');
    }
  }

  function restoreBackup(row: BackupDto) {
    modal.confirm({
      title: '确认恢复该备份？',
      content: `系统会先创建当前数据库快照，再用该备份覆盖当前数据库。备份时间：${row.createdAt}`,
      okText: '恢复',
      okButtonProps: { danger: true },
      onOk: async () => {
        try {
          const result = await api.restoreBackup(row.id);
          message.success(result.message);
          await refresh();
        } catch (error) {
          message.error(error instanceof Error ? error.message : '恢复失败');
        }
      }
    });
  }

  async function saveSettings() {
    try {
      await api.saveSettings(settingsForm.getFieldsValue());
      message.success('设置已保存');
      await refresh();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '设置保存失败');
    }
  }

  async function runSelfCheck() {
    try {
      const result = await api.runDataSelfCheck();
      setSelfCheck(result);
      message.success(result.issueCount === 0 ? '数据自检通过' : `发现 ${result.issueCount} 个异常`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '数据自检失败');
    }
  }

  async function exportSelfCheck() {
    modal.confirm({
      title: '导出自检结果前请确认隐私边界',
      content: '自检结果可能包含订单号、单据文件名和异常摘要。导出前系统会尽量脱敏，但提交给他人前仍建议检查内容。',
      okText: '导出',
      onOk: async () => {
        try {
          const path = await api.exportDataSelfCheck();
          message.success(`自检结果已导出：${finalPathComponent(path)}`);
        } catch (error) {
          message.error(error instanceof Error ? error.message : '导出自检结果失败');
        }
      }
    });
  }

  async function exportDiagnosticPackage() {
    modal.confirm({
      title: '导出诊断包前请确认隐私边界',
      content: '诊断包可能包含运行日志和本机路径信息。导出前系统会尽量脱敏，但提交给他人前仍建议检查内容，避免包含客户、电话、地址、真实订单或库存金额。',
      okText: '导出',
      onOk: async () => {
        try {
          const result = await api.exportDiagnosticPackage();
          message.success(`${result.message}：${finalPathComponent(result.filePath)}`);
        } catch (error) {
          message.error(error instanceof Error ? error.message : '导出诊断包失败');
        }
      }
    });
  }

  useEffect(() => {
    void refresh();
  }, []);

  useEffect(() => {
    setGenericHeaders(null);
    setVisualFieldMapping({});
    setGenericPreview(null);
    setGenericResult(null);
  }, [genericImportType]);

  return (
    <div className="page">
      <div className="page-title"><Typography.Title level={2}>系统设置</Typography.Title></div>
      <LocalPathsCard appStatus={appStatus} />
      <SetupGuideCard onReopen={reopenSetup} />
      <MerchantProfileCard form={merchantForm} merchant={merchant} onSave={() => void saveMerchant()} />
      <IndustryFeatureCard
        form={featuresForm}
        industryTemplates={industryTemplates}
        onApplyTemplate={applyIndustryTemplate}
        onSaveFeatures={() => void saveFeatures()}
      />
      <TermSettingsCard form={termsForm} onSave={() => void saveTerms()} />
      <DocumentTemplateSettingsCard
        form={settingsForm}
        documentTemplates={documentTemplates}
        merchant={merchant}
        terms={terms}
        printers={printers}
        templateValues={templateValues}
        onSave={() => void saveSettings()}
        onApplyTemplate={(templateId) => void applyDocumentTemplate(templateId)}
        onOpenExportsFolder={() => void api.openExportsFolder()}
        onOpenLogsFolder={() => void api.openLogsFolder()}
      />
      <Form form={genericImportForm} layout="vertical" className="dense-form">
        <Card title="通用数据导入">
          <Alert
            type="info"
            showIcon
            style={{ marginBottom: 12 }}
            message="通用导入不会清空历史订单或业务流水。导入前可预览新增、覆盖、跳过和异常行。"
          />
          <Space.Compact block>
            <Form.Item label="导入类型" name="importType" rules={[{ required: true, message: '请选择导入类型' }]} style={{ width: '22%' }}>
              <Select options={genericImportTypeOptions} />
            </Form.Item>
            <Form.Item label="Excel 文件路径" name="filePath" rules={[{ required: true, message: '请输入 Excel 文件路径' }]} style={{ width: '50%' }}>
              <Input placeholder="例如 C:/Users/ww/Desktop/work/商品导入.xlsx" />
            </Form.Item>
            <Form.Item label="重复数据处理" name="duplicateStrategy" style={{ width: '14%' }}>
              <Select
                options={[
                  { value: 'skip', label: '跳过' },
                  { value: 'overwrite', label: '覆盖' },
                  { value: 'append_suffix', label: '新增并追加后缀' }
                ]}
              />
            </Form.Item>
            <Form.Item label="映射方案名" name="mappingName" style={{ width: '14%' }}>
              <Input placeholder={currentMapping?.name ?? '可选'} />
            </Form.Item>
          </Space.Compact>
          <Form.Item label="字段映射 JSON" name="fieldMappingText">
            <Input.TextArea
              rows={3}
              placeholder='高级可选，例如 {"商品名称":"品名","客户名称":"客户单位"}；留空时按系统内置表头和上方可视化映射识别。'
            />
          </Form.Item>
          <Space style={{ marginBottom: 12 }}>
            <Button onClick={() => void previewGenericImportHeaders()}>读取表头</Button>
            <Button disabled={!currentMapping} onClick={applySavedMapping}>套用已保存映射</Button>
            {genericHeaders && (
              <Tag color="blue">
                工作表：{genericHeaders.sheetName} / 表头 {genericHeaders.headers.length} 个
              </Tag>
            )}
            {missingRequiredMapping.length > 0 && (
              <Tag color="red">缺少必填映射：{missingRequiredMapping.join('、')}</Tag>
            )}
          </Space>
          {genericHeaders && (
            <Table
              rowKey="name"
              size="small"
              style={{ marginBottom: 12 }}
              dataSource={genericHeaders.fields}
              pagination={false}
              columns={[
                { title: '系统字段', dataIndex: 'name', width: 140 },
                {
                  title: '必填',
                  width: 80,
                  render: (_, row) => row.required ? <Tag color="red">必填</Tag> : <Tag>可选</Tag>
                },
                {
                  title: 'Excel 列',
                  render: (_, row) => (
                    <Select
                      allowClear
                      showSearch
                      optionFilterProp="label"
                      placeholder="选择 Excel 表头"
                      value={visualFieldMapping[row.name]}
                      style={{ width: 260 }}
                      options={genericHeaders.headers.map((header) => ({ value: header, label: header }))}
                      onChange={(value) => setVisualFieldMapping((mapping) => {
                        const next = { ...mapping };
                        if (value) {
                          next[row.name] = value;
                        } else {
                          delete next[row.name];
                        }
                        return next;
                      })}
                    />
                  )
                },
                {
                  title: '可识别别名',
                  render: (_, row) => row.aliases.join('、') || '-'
                }
              ]}
            />
          )}
          <Space style={{ marginBottom: 12 }}>
            <Button onClick={() => void downloadImportTemplate()}>下载模板</Button>
            <Button onClick={() => void previewGenericImport()}>预览导入</Button>
            <Button
              type="primary"
              disabled={!genericPreview || genericPreview.validCount === 0}
              onClick={() => modal.confirm({
                title: '确认写入通用导入数据？',
                content: '本操作只写入预览中有效的数据，不会清空已有订单、客户、库存流水或单据档案。',
                okText: '确认导入',
                onOk: () => confirmGenericImport()
              })}
            >
              确认导入
            </Button>
            <Button onClick={() => void saveCurrentMapping()}>保存映射方案</Button>
            <Button disabled={!genericPreview && !genericResult} onClick={() => void exportGenericImportReport()}>导出报告</Button>
          </Space>
          <div className="stat-grid">
            <Statistic title="总行数" value={genericResult?.rows.length ?? genericPreview?.totalCount ?? 0} />
            <Statistic title="有效" value={genericPreview?.validCount ?? 0} />
            <Statistic title="新增" value={genericResult?.createCount ?? genericPreview?.createCount ?? 0} />
            <Statistic title="覆盖" value={genericResult?.overwriteCount ?? genericPreview?.overwriteCount ?? 0} />
            <Statistic title="异常" value={genericResult?.errorCount ?? genericPreview?.errorCount ?? 0} />
          </div>
          <Table
            rowKey={(row) => `${row.rowNumber}-${row.name ?? row.barcode ?? row.status}`}
            size="small"
            style={{ marginTop: 12 }}
            dataSource={genericResult?.rows ?? genericPreview?.rows ?? []}
            pagination={{ pageSize: 8 }}
            scroll={{ x: 1100 }}
            columns={[
              { title: '行号', dataIndex: 'rowNumber', width: 70 },
              { title: '名称', dataIndex: 'name', width: 160 },
              { title: '类别', dataIndex: 'category', width: 110 },
              { title: '地区', dataIndex: 'region', width: 110 },
              { title: '条码', dataIndex: 'barcode', width: 130 },
              { title: '数量', dataIndex: 'quantity', width: 90 },
              { title: '单价/成本', dataIndex: 'unitPrice', width: 100 },
              { title: '地址', dataIndex: 'address', width: 180 },
              { title: '动作', render: (_, row) => <Tag>{actionText(row.action)}</Tag>, width: 90 },
              { title: '状态', render: (_, row) => <Tag color={statusColor(row.status)}>{statusText(row.status)}</Tag>, width: 90 },
              { title: '说明', dataIndex: 'message', width: 220 }
            ]}
          />
        </Card>
      </Form>
      <Collapse
        items={[
          {
            key: 'legacy',
            label: '高级：历史兼容迁移',
            children: (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Alert
                  type="warning"
                  showIcon
                  message="历史兼容迁移只用于原固定结构工作簿。执行前会自动备份当前数据库，但迁移会清空并重建商品、客户、订单、库存流水、规则、额度和单据等业务表。日常追加导入请使用上方通用数据导入。"
                />
                <Space.Compact style={{ width: '100%' }}>
                  <Input
                    value={legacyExcelPath}
                    onChange={(event) => setLegacyExcelPath(event.target.value)}
                    placeholder="历史 Excel 文件路径"
                  />
                  <Button
                    danger
                    onClick={() => modal.confirm({
                      title: '确认执行历史兼容迁移？',
                      content: '该入口会先自动备份当前数据库，然后清空并重建业务表。请确认你正在迁移原固定结构 Excel 工作簿，而不是做日常追加导入。',
                      okText: '执行迁移',
                      okButtonProps: { danger: true },
                      onOk: () => importLegacyExcel()
                    })}
                  >
                    兼容迁移
                  </Button>
                </Space.Compact>
                {legacyImportResult && (
                  <List
                    size="small"
                    header={`商品 ${legacyImportResult.productCount} / 客户 ${legacyImportResult.customerCount} / 流水 ${legacyImportResult.movementCount}`}
                    dataSource={[...legacyImportResult.warnings, ...legacyImportResult.errors]}
                    renderItem={(item) => <List.Item>{item}</List.Item>}
                  />
                )}
              </Space>
            )
          }
        ]}
      />
      <BackupRestoreCard
        backups={backups}
        onBackup={() => void backup()}
        onOpenBackupFolder={() => void api.openBackupFolder()}
        onRestoreBackup={restoreBackup}
      />
      {features.diagnostics && (
        <DiagnosticsCard
          appStatus={appStatus}
          backups={backups}
          diagnostic={diagnostic}
          selfCheck={selfCheck}
          onRunSelfCheck={() => void runSelfCheck()}
          onExportSelfCheck={() => void exportSelfCheck()}
          onExportDiagnosticPackage={() => void exportDiagnosticPackage()}
          onRefresh={() => void refresh()}
        />
      )}
      <AuditLogCard auditLogs={auditLogs} />
    </div>
  );
}

function actionText(action: string) {
  if (action === 'create') {
    return '新增';
  }
  if (action === 'overwrite') {
    return '覆盖';
  }
  if (action === 'append_suffix') {
    return '追加后缀';
  }
  return '跳过';
}

function genericImportTypeText(value?: string) {
  return genericImportTypeOptions.find((item) => item.value === value)?.label ?? '通用导入';
}

function statusText(status: string) {
  if (status === 'valid') {
    return '有效';
  }
  if (status === 'imported') {
    return '已导入';
  }
  if (status === 'error') {
    return '异常';
  }
  return '跳过';
}

function statusColor(status: string) {
  if (status === 'valid') {
    return 'blue';
  }
  if (status === 'imported') {
    return 'green';
  }
  if (status === 'error') {
    return 'red';
  }
  return 'default';
}

function settingsToForm(settings: SettingDto[], merchantName: string) {
  const map = Object.fromEntries(settings.map((item) => [item.key, item.value]));
  return {
    dailyAutoBackup: map.daily_auto_backup !== 'false',
    defaultPrintTemplate: map.default_print_template || 'general',
    defaultExportFormat: map.default_export_format || 'xlsx',
    defaultPrinter: map.default_printer || undefined,
    templateStoreName: map.template_store_name || merchantName || '我的商行',
    templateFooterText: map.template_footer_text || '',
    templateShowBarcode: map.template_show_barcode !== 'false',
    templateProductLabel: map.template_product_label || '商品名称',
    templateQuantityLabel: map.template_quantity_label || '数量',
    templatePriceLabel: map.template_price_label || '价格',
    templateAmountLabel: map.template_amount_label || '总价格',
    templateRemarkLabel: map.template_remark_label || '备注',
    templateOrientation: map.template_orientation || 'portrait',
    templateMargin: Number(map.template_margin || 0)
  };
}

function parseFieldMapping(text?: string): Record<string, string> | undefined {
  const trimmed = text?.trim();
  if (!trimmed) {
    return undefined;
  }
  const parsed = JSON.parse(trimmed) as unknown;
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('字段映射必须是 JSON 对象');
  }
  return Object.fromEntries(
    Object.entries(parsed as Record<string, unknown>).map(([key, value]) => [key, String(value)])
  );
}

function mergeFieldMapping(
  visualMapping: Record<string, string>,
  mappingText?: string
): Record<string, string> | undefined {
  const fromText = parseFieldMapping(mappingText) ?? {};
  const cleanedVisual = Object.fromEntries(
    Object.entries(visualMapping)
      .map(([key, value]) => [key.trim(), value.trim()])
      .filter(([key, value]) => key && value)
  );
  const merged = { ...cleanedVisual, ...fromText };
  return Object.keys(merged).length > 0 ? merged : undefined;
}

function finalPathComponent(path: string) {
  const normalized = path.replace(/\\/g, '/');
  const parts = normalized.split('/').filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : path;
}
