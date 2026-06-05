import {
  Alert,
  App,
  Button,
  Card,
  Collapse,
  Descriptions,
  Form,
  Input,
  InputNumber,
  List,
  Select,
  Space,
  Statistic,
  Switch,
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

const featureItems: Array<{ name: keyof FeatureFlagsDto; label: string }> = [
  { name: 'supplierLedger', label: '供应商采购台账' },
  { name: 'customerRules', label: '价格规则' },
  { name: 'monthlyCredit', label: '返利额度' },
  { name: 'receivables', label: '欠款收款' },
  { name: 'productRanking', label: '商品经营排行' },
  { name: 'customerAnalysis', label: '客户经营分析' },
  { name: 'inventoryControl', label: '库存盘点' },
  { name: 'diagnostics', label: '诊断中心' }
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
    try {
      const path = await api.exportDataSelfCheck();
      message.success(`自检结果已导出：${path}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导出自检结果失败');
    }
  }

  async function exportDiagnosticPackage() {
    try {
      const result = await api.exportDiagnosticPackage();
      message.success(`${result.message}：${result.filePath}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导出诊断包失败');
    }
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
      <Card title="本地路径">
        <Descriptions column={1} size="small">
          <Descriptions.Item label="数据库">{appStatus?.databasePath}</Descriptions.Item>
          <Descriptions.Item label="数据目录">{appStatus?.dataDir}</Descriptions.Item>
          <Descriptions.Item label="订单目录">{appStatus?.ordersDir}</Descriptions.Item>
          <Descriptions.Item label="导出目录">{appStatus?.exportsDir}</Descriptions.Item>
          <Descriptions.Item label="备份目录">{appStatus?.backupsDir}</Descriptions.Item>
          <Descriptions.Item label="日志目录">{appStatus?.logsDir}</Descriptions.Item>
          <Descriptions.Item label="版本">{appStatus?.version}</Descriptions.Item>
        </Descriptions>
      </Card>
      <Card title="初始化向导">
        <Space direction="vertical" style={{ width: '100%' }}>
          <Alert
            type="info"
            showIcon
            message="可重新打开初始化向导调整商户、行业模板、术语、功能开关和默认单据设置。该操作不会清空已有业务数据。"
          />
          <Button onClick={reopenSetup}>重新打开初始化向导</Button>
        </Space>
      </Card>
      <Form form={merchantForm} layout="vertical" className="dense-form">
        <Card title="商户信息">
          <Space.Compact block>
            <Form.Item label="商户名称" name="name" rules={[{ required: true, message: '请输入商户名称' }]} style={{ width: '25%' }}>
              <Input />
            </Form.Item>
            <Form.Item label="联系人" name="contact" style={{ width: '25%' }}>
              <Input />
            </Form.Item>
            <Form.Item label="电话" name="phone" style={{ width: '25%' }}>
              <Input />
            </Form.Item>
            <Form.Item label="地址" name="address" style={{ width: '25%' }}>
              <Input />
            </Form.Item>
          </Space.Compact>
          <Space.Compact block>
            <Form.Item label="Logo 路径" name="logoPath" style={{ width: '50%' }}>
              <Input placeholder="可选，例如 C:/Users/ww/Desktop/logo.png" />
            </Form.Item>
            <Form.Item label="备注" name="remark" style={{ width: '50%' }}>
              <Input />
            </Form.Item>
          </Space.Compact>
          <Space>
            <Button type="primary" onClick={() => void saveMerchant()}>保存商户信息</Button>
            <Typography.Text type="secondary">当前单据抬头：{merchant.name || '我的商行'}</Typography.Text>
          </Space>
        </Card>
      </Form>
      <Card title="行业模板与功能开关">
        <Table
          rowKey="id"
          size="small"
          dataSource={industryTemplates}
          pagination={false}
          columns={[
            { title: '模板', dataIndex: 'name', width: 140 },
            { title: '说明', dataIndex: 'description' },
            { title: '默认单据', dataIndex: 'orderTemplate', width: 120 },
            {
              title: '操作',
              width: 100,
              render: (_, row) => <Button size="small" onClick={() => applyIndustryTemplate(row)}>应用</Button>
            }
          ]}
        />
        <Form form={featuresForm} layout="vertical" className="dense-form" style={{ marginTop: 12 }}>
          <div className="feature-grid">
            {featureItems.map((item) => (
              <Form.Item key={item.name} label={item.label} name={item.name} valuePropName="checked">
                <Switch />
              </Form.Item>
            ))}
          </div>
          <Button onClick={() => void saveFeatures()}>保存功能开关</Button>
        </Form>
      </Card>
      <Form form={termsForm} layout="vertical" className="dense-form">
        <Card title="业务术语">
          <Space.Compact block>
            <Form.Item label="客户显示名" name="customer" style={{ width: '25%' }}><Input /></Form.Item>
            <Form.Item label="地区显示名" name="region" style={{ width: '25%' }}><Input /></Form.Item>
            <Form.Item label="商品显示名" name="product" style={{ width: '25%' }}><Input /></Form.Item>
            <Form.Item label="类别显示名" name="category" style={{ width: '25%' }}><Input /></Form.Item>
          </Space.Compact>
          <Space.Compact block>
            <Form.Item label="规则显示名" name="rule" style={{ width: '33.33%' }}><Input /></Form.Item>
            <Form.Item label="额度显示名" name="credit" style={{ width: '33.33%' }}><Input /></Form.Item>
            <Form.Item label="默认客户显示名" name="guestCustomer" style={{ width: '33.33%' }}><Input /></Form.Item>
          </Space.Compact>
          <Button type="primary" onClick={() => void saveTerms()}>保存术语配置</Button>
        </Card>
      </Form>
      <Form form={settingsForm} layout="vertical" className="dense-form">
        <Card title="系统与单据模板设置">
          <Space.Compact block>
            <Form.Item label="每日自动备份" name="dailyAutoBackup" valuePropName="checked" style={{ width: '25%' }}>
              <Switch />
            </Form.Item>
            <Form.Item label="默认打印模板" name="defaultPrintTemplate" style={{ width: '25%' }}>
              <Select options={documentTemplates.map((item) => ({ value: item.id, label: item.name }))} />
            </Form.Item>
            <Form.Item label="默认导出格式" name="defaultExportFormat" style={{ width: '25%' }}>
              <Select options={[{ value: 'xlsx', label: 'Excel xlsx' }]} />
            </Form.Item>
            <Form.Item label="默认打印机" name="defaultPrinter" style={{ width: '25%' }}>
              <Select allowClear options={printers.map((item) => ({ value: item, label: item }))} />
            </Form.Item>
          </Space.Compact>
          <Space.Compact block>
            <Form.Item label="店名" name="templateStoreName" style={{ width: '25%' }}>
              <Input />
            </Form.Item>
            <Form.Item label="页脚/默认备注" name="templateFooterText" style={{ width: '25%' }}>
              <Input />
            </Form.Item>
            <Form.Item label="纸张方向" name="templateOrientation" style={{ width: '20%' }}>
              <Select options={[{ value: 'portrait', label: '纵向' }, { value: 'landscape', label: '横向' }]} />
            </Form.Item>
            <Form.Item label="页边距" name="templateMargin" style={{ width: '15%' }}>
              <InputNumber min={0} max={2} step={0.1} style={{ width: '100%' }} />
            </Form.Item>
            <Form.Item label="显示条码" name="templateShowBarcode" valuePropName="checked" style={{ width: '15%' }}>
              <Switch />
            </Form.Item>
          </Space.Compact>
          <Space.Compact block>
            <Form.Item label="商品列名" name="templateProductLabel" style={{ width: '20%' }}><Input /></Form.Item>
            <Form.Item label="数量列名" name="templateQuantityLabel" style={{ width: '20%' }}><Input /></Form.Item>
            <Form.Item label="价格列名" name="templatePriceLabel" style={{ width: '20%' }}><Input /></Form.Item>
            <Form.Item label="金额列名" name="templateAmountLabel" style={{ width: '20%' }}><Input /></Form.Item>
            <Form.Item label="备注列名" name="templateRemarkLabel" style={{ width: '20%' }}><Input /></Form.Item>
          </Space.Compact>
          <div className="template-preview">
            <div className="template-preview-title">{templateValues.templateStoreName || merchant.name || '我的商行'}</div>
            <div className="template-preview-meta">
              {terms.customer}：测试{terms.customer}　单号：20260601001　方向：{templateValues.templateOrientation === 'landscape' ? '横向' : '纵向'}
            </div>
            <div className="template-preview-row">
              <span>{templateValues.templateShowBarcode === false ? '' : '条码'}</span>
              <span>{templateValues.templateProductLabel || '商品名称'}</span>
              <span>{templateValues.templateQuantityLabel || '数量'}</span>
              <span>{templateValues.templatePriceLabel || '价格'}</span>
              <span>{templateValues.templateAmountLabel || '总价格'}</span>
              <span>{templateValues.templateRemarkLabel || '备注'}</span>
            </div>
            <div className="template-preview-footer">{templateValues.templateFooterText || '页脚/默认备注'}</div>
          </div>
          <Space>
            <Button type="primary" onClick={() => void saveSettings()}>保存设置</Button>
            <Button
              onClick={() => settingsForm.setFieldsValue({
                templateStoreName: merchant.name || '我的商行',
                templateFooterText: '',
                templateShowBarcode: true,
                templateProductLabel: '商品名称',
                templateQuantityLabel: '数量',
                templatePriceLabel: '价格',
                templateAmountLabel: '总价格',
                templateRemarkLabel: '备注',
                templateOrientation: 'portrait',
                templateMargin: 0
              })}
            >
              恢复默认模板
            </Button>
            {documentTemplates.map((item) => (
              <Button key={item.id} onClick={() => void applyDocumentTemplate(item.id)}>{item.name}</Button>
            ))}
            <Button onClick={() => void api.openExportsFolder()}>打开导出目录</Button>
            <Button onClick={() => void api.openLogsFolder()}>打开日志目录</Button>
          </Space>
        </Card>
      </Form>
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
      <Card title="备份与恢复">
        <Space>
          <Button onClick={() => void backup()}>立即备份</Button>
          <Button onClick={() => void api.openBackupFolder()}>打开备份目录</Button>
        </Space>
        <Table
          rowKey="id"
          size="small"
          style={{ marginTop: 12 }}
          dataSource={backups}
          columns={[
            { title: '时间', dataIndex: 'createdAt', width: 160 },
            { title: '类型', dataIndex: 'backupType', width: 120 },
            { title: '状态', render: (_, row) => statusTag(row.status), width: 100 },
            { title: '路径', dataIndex: 'backupPath' },
            {
              title: '操作',
              width: 100,
              render: (_, row) => (
                <Button size="small" danger disabled={row.status !== 'success'} onClick={() => restoreBackup(row)}>
                  恢复
                </Button>
              )
            }
          ]}
        />
      </Card>
      {features.diagnostics && (
        <Card title="诊断中心">
          <Space style={{ marginBottom: 12 }}>
            <Button type="primary" onClick={() => void runSelfCheck()}>运行数据自检</Button>
            <Button onClick={() => void exportSelfCheck()}>导出自检结果</Button>
            <Button onClick={() => void exportDiagnosticPackage()}>导出诊断包</Button>
            <Button onClick={() => void refresh()}>刷新诊断信息</Button>
          </Space>
          <Descriptions column={2} size="small">
            <Descriptions.Item label="数据库">{diagnostic?.databasePath ?? appStatus?.databasePath}</Descriptions.Item>
            <Descriptions.Item label="数据库大小">{diagnostic?.databaseSize ?? 0} B</Descriptions.Item>
            <Descriptions.Item label="版本">{diagnostic?.version ?? appStatus?.version}</Descriptions.Item>
            <Descriptions.Item label="备份数量">{diagnostic?.backupCount ?? backups.length}</Descriptions.Item>
            <Descriptions.Item label="最近备份">{diagnostic?.latestBackupAt ?? '-'}</Descriptions.Item>
            <Descriptions.Item label="基础统计">
              商品 {diagnostic?.productCount ?? 0} / 客户 {diagnostic?.customerCount ?? 0} / 订单 {diagnostic?.orderCount ?? 0} / 单据 {diagnostic?.documentCount ?? 0}
            </Descriptions.Item>
          </Descriptions>
          {selfCheck && (
            <div style={{ marginTop: 12 }}>
              <Descriptions column={4} size="small">
                <Descriptions.Item label="自检时间">{selfCheck.checkedAt}</Descriptions.Item>
                <Descriptions.Item label="异常数">{selfCheck.issueCount}</Descriptions.Item>
                <Descriptions.Item label="库存">{selfCheck.inventoryChecked}</Descriptions.Item>
                <Descriptions.Item label="订单">{selfCheck.ordersChecked}</Descriptions.Item>
                <Descriptions.Item label="月费">{selfCheck.creditsChecked}</Descriptions.Item>
                <Descriptions.Item label="单据">{selfCheck.documentsChecked}</Descriptions.Item>
              </Descriptions>
              <Table
                rowKey={(row) => `${row.checkCode}-${row.targetType}-${row.targetId ?? row.targetLabel}`}
                size="small"
                dataSource={selfCheck.issues}
                columns={[
                  { title: '级别', render: (_, row) => statusTag(row.severity), width: 90 },
                  { title: '检查项', dataIndex: 'checkCode', width: 170 },
                  { title: '对象', dataIndex: 'targetLabel', width: 180 },
                  { title: '说明', dataIndex: 'message' },
                  { title: '详情', dataIndex: 'details' }
                ]}
              />
            </div>
          )}
          <Card size="small" title="最近日志" style={{ marginTop: 12 }}>
            <List
              size="small"
              dataSource={diagnostic?.latestLogs ?? []}
              renderItem={(item) => <List.Item>{item}</List.Item>}
            />
          </Card>
        </Card>
      )}
      <Card title="审计日志">
        <Table
          rowKey="id"
          size="small"
          dataSource={auditLogs}
          columns={[
            { title: '时间', dataIndex: 'logTime', width: 170 },
            { title: '模块', dataIndex: 'module', width: 110 },
            { title: '动作', dataIndex: 'action', width: 140 },
            { title: '对象', dataIndex: 'targetLabel' },
            { title: '结果', render: (_, row) => statusTag(row.result), width: 90 },
            { title: '说明', dataIndex: 'message' }
          ]}
        />
      </Card>
    </div>
  );
}

function statusTag(status: string) {
  if (status === 'success' || status === 'normal') {
    return <Tag color="green">{status}</Tag>;
  }
  if (status === 'warning') {
    return <Tag color="orange">{status}</Tag>;
  }
  if (status === 'voided' || status === 'failed' || status === 'error') {
    return <Tag color="red">{status}</Tag>;
  }
  return <Tag>{status}</Tag>;
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
