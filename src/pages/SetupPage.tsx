import { Alert, App, Button, Card, Form, Input, Select, Space, Steps, Switch, Typography } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { api } from '../api/inventory';
import type {
  DocumentTemplateDto,
  FeatureFlagsDto,
  IndustryTemplateDto,
  MerchantProfileDto,
  SettingDto,
  TermSettingsDto
} from '../shared/types';
import { defaultFeatures, defaultMerchant, defaultTerms, useAppStore } from '../store/appStore';

type SetupFormValues = {
  merchant: MerchantProfileDto;
  industryTemplate?: string;
  terms: TermSettingsDto;
  features: FeatureFlagsDto;
  defaultPrintTemplate?: string;
  defaultExportFormat?: string;
  defaultPrinter?: string;
  importPlan?: 'skip' | 'generic' | 'legacy';
};

const setupSteps = [
  { title: '商户信息' },
  { title: '行业模板' },
  { title: '术语配置' },
  { title: '单据与功能' },
  { title: '数据导入' },
  { title: '完成初始化' }
];

export default function SetupPage() {
  const { message } = App.useApp();
  const navigate = useNavigate();
  const [form] = Form.useForm<SetupFormValues>();
  const {
    setSetupStatus,
    setMerchant,
    setTerms,
    setFeatures
  } = useAppStore();
  const [current, setCurrent] = useState(0);
  const [templates, setTemplates] = useState<IndustryTemplateDto[]>([]);
  const [documentTemplates, setDocumentTemplates] = useState<DocumentTemplateDto[]>([]);
  const [printers, setPrinters] = useState<string[]>([]);
  const selectedTemplateId = Form.useWatch('industryTemplate', form);
  const importPlan = Form.useWatch('importPlan', form);
  const selectedTemplate = useMemo(
    () => templates.find((item) => item.id === selectedTemplateId),
    [selectedTemplateId, templates]
  );

  useEffect(() => {
    async function load() {
      const [
        nextTemplates,
        nextPrinters,
        nextDocumentTemplates,
        nextSettings,
        setupStatus,
        merchant,
        terms,
        features
      ] = await Promise.all([
        api.industryTemplates(),
        api.printers().catch(() => [] as string[]),
        api.documentTemplates().catch(() => [] as DocumentTemplateDto[]),
        api.settings().catch(() => [] as SettingDto[]),
        api.setupStatus().catch(() => undefined),
        api.merchantProfile().catch(() => defaultMerchant),
        api.termSettings().catch(() => defaultTerms),
        api.featureFlags().catch(() => defaultFeatures)
      ]);
      const settings = Object.fromEntries(nextSettings.map((item) => [item.key, item.value]));
      setTemplates(nextTemplates);
      setPrinters(nextPrinters);
      setDocumentTemplates(nextDocumentTemplates);
      const currentTemplate = nextTemplates.find((item) => item.id === setupStatus?.industryTemplate) ?? nextTemplates[0];
      form.setFieldsValue({
        merchant,
        industryTemplate: currentTemplate?.id ?? 'general_wholesale',
        terms,
        features,
        defaultPrintTemplate: settings.default_print_template || settings.active_order_template || currentTemplate?.orderTemplate || 'general',
        defaultExportFormat: settings.default_export_format || 'xlsx',
        defaultPrinter: settings.default_printer || undefined,
        importPlan: 'skip',
      });
    }
    void load();
  }, [form]);

  function applyTemplate(templateId?: string) {
    const template = templates.find((item) => item.id === templateId);
    if (!template) {
      return;
    }
    form.setFieldsValue({
      terms: template.terms,
      features: template.features,
      defaultPrintTemplate: template.orderTemplate
    });
  }

  async function nextStep() {
    await form.validateFields(stepFields(current));
    setCurrent((value) => Math.min(value + 1, setupSteps.length - 1));
  }

  async function complete() {
    try {
      await form.validateFields();
      const values = form.getFieldsValue(true);
      await api.completeSetup({
        merchant: values.merchant,
        terms: values.terms,
        features: values.features,
        industryTemplate: values.industryTemplate,
        defaultPrintTemplate: values.defaultPrintTemplate,
        defaultExportFormat: values.defaultExportFormat,
        defaultPrinter: values.defaultPrinter
      });
      const [nextSetup, nextMerchant, nextTerms, nextFeatures] = await Promise.all([
        api.setupStatus(),
        api.merchantProfile(),
        api.termSettings(),
        api.featureFlags()
      ]);
      setSetupStatus(nextSetup);
      setMerchant(nextMerchant);
      setTerms(nextTerms);
      setFeatures(nextFeatures);
      message.success('初始化完成');
      navigate('/', { replace: true });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '初始化失败');
    }
  }

  async function downloadTemplate(importType: 'products' | 'customers' | 'initial_stock') {
    try {
      const path = await api.downloadImportTemplate(importType);
      message.success(`导入模板已导出：${path}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导入模板导出失败');
    }
  }

  return (
    <div className="page setup-page">
      <div className="page-title">
        <Typography.Title level={2}>首次使用初始化</Typography.Title>
      </div>
      <Card>
        <Steps current={current} items={setupSteps} />
      </Card>
      <Form form={form} layout="vertical" className="dense-form">
        {current === 0 && (
          <Card title="商户信息">
            <Space.Compact block>
              <Form.Item
                label="商户名称"
                name={['merchant', 'name']}
                rules={[{ required: true, message: '请输入商户名称' }]}
                style={{ width: '25%' }}
              >
                <Input placeholder="例如：我的商行" />
              </Form.Item>
              <Form.Item label="联系人" name={['merchant', 'contact']} style={{ width: '25%' }}>
                <Input />
              </Form.Item>
              <Form.Item label="电话" name={['merchant', 'phone']} style={{ width: '25%' }}>
                <Input />
              </Form.Item>
              <Form.Item label="地址" name={['merchant', 'address']} style={{ width: '25%' }}>
                <Input />
              </Form.Item>
            </Space.Compact>
            <Space.Compact block>
              <Form.Item label="Logo 路径" name={['merchant', 'logoPath']} style={{ width: '50%' }}>
                <Input placeholder="可选，例如 C:/Users/ww/Desktop/logo.png" />
              </Form.Item>
              <Form.Item label="备注" name={['merchant', 'remark']} style={{ width: '50%' }}>
                <Input />
              </Form.Item>
            </Space.Compact>
          </Card>
        )}
        {current === 1 && (
          <Card title="行业模板">
            <Form.Item label="选择行业模板" name="industryTemplate">
              <Select
                options={templates.map((item) => ({ value: item.id, label: item.name }))}
                onChange={applyTemplate}
              />
            </Form.Item>
            <Typography.Paragraph type="secondary">
              {selectedTemplate?.description ?? '行业模板会设置默认术语、功能入口和单据模板。'}
            </Typography.Paragraph>
          </Card>
        )}
        {current === 2 && (
          <Card title="术语配置">
            <Space.Compact block>
              <Form.Item label="客户显示名" name={['terms', 'customer']} style={{ width: '25%' }}>
                <Input />
              </Form.Item>
              <Form.Item label="地区显示名" name={['terms', 'region']} style={{ width: '25%' }}>
                <Input />
              </Form.Item>
              <Form.Item label="商品显示名" name={['terms', 'product']} style={{ width: '25%' }}>
                <Input />
              </Form.Item>
              <Form.Item label="类别显示名" name={['terms', 'category']} style={{ width: '25%' }}>
                <Input />
              </Form.Item>
            </Space.Compact>
            <Space.Compact block>
              <Form.Item label="规则显示名" name={['terms', 'rule']} style={{ width: '33.33%' }}>
                <Input />
              </Form.Item>
              <Form.Item label="额度显示名" name={['terms', 'credit']} style={{ width: '33.33%' }}>
                <Input />
              </Form.Item>
              <Form.Item label="默认客户显示名" name={['terms', 'guestCustomer']} style={{ width: '33.33%' }}>
                <Input />
              </Form.Item>
            </Space.Compact>
          </Card>
        )}
        {current === 3 && (
          <Card title="单据与功能">
            <Space.Compact block>
              <Form.Item label="默认打印模板" name="defaultPrintTemplate" style={{ width: '33.33%' }}>
                <Select
                  options={documentTemplates.length > 0
                    ? documentTemplates
                      .filter((item) => item.templateType === 'order')
                      .map((item) => ({ value: item.id, label: item.name }))
                    : [
                      { value: 'general', label: '通用出库单' },
                      { value: 'delivery', label: '配送出库单' },
                      { value: 'kezhan_legacy', label: '科展兼容模板' },
                      { value: 'simple', label: '简洁出库单' }
                    ]}
                />
              </Form.Item>
              <Form.Item label="默认导出格式" name="defaultExportFormat" style={{ width: '33.33%' }}>
                <Select options={[{ value: 'xlsx', label: 'Excel xlsx' }]} />
              </Form.Item>
              <Form.Item label="默认打印机" name="defaultPrinter" style={{ width: '33.33%' }}>
                <Select allowClear options={printers.map((item) => ({ value: item, label: item }))} />
              </Form.Item>
            </Space.Compact>
            <div className="feature-grid">
              {featureItems.map((item) => (
                <Form.Item
                  key={item.name}
                  label={item.label}
                  name={['features', item.name]}
                  valuePropName="checked"
                >
                  <Switch />
                </Form.Item>
              ))}
            </div>
          </Card>
        )}
        {current === 4 && (
          <Card title="数据导入">
            <Alert
              type="info"
              showIcon
              style={{ marginBottom: 12 }}
              message="初始化阶段不会直接写入业务数据。你可以先下载通用模板，完成初始化后到系统设置中的“通用数据导入”区域预览并确认导入。"
            />
            <Form.Item label="初始化后的导入方式" name="importPlan">
              <Select
                options={[
                  { value: 'skip', label: '暂不导入，先进入系统' },
                  { value: 'generic', label: '使用通用 Excel 模板导入' },
                  { value: 'legacy', label: '使用历史兼容迁移' }
                ]}
              />
            </Form.Item>
            {importPlan === 'generic' && (
              <Space wrap>
                <Button onClick={() => void downloadTemplate('products')}>下载商品模板</Button>
                <Button onClick={() => void downloadTemplate('customers')}>下载客户模板</Button>
                <Button onClick={() => void downloadTemplate('initial_stock')}>下载期初库存模板</Button>
              </Space>
            )}
            {importPlan === 'legacy' && (
              <Alert
                type="warning"
                showIcon
                style={{ marginTop: 12 }}
                message="历史兼容迁移仅适用于原固定结构工作簿。完成初始化后请到系统设置的高级区域执行，系统会在迁移前自动备份数据库。"
              />
            )}
            {importPlan === 'skip' && (
              <Typography.Paragraph type="secondary">
                跳过导入后仍可正常新增{form.getFieldValue(['terms', 'product']) || '商品'}、{form.getFieldValue(['terms', 'customer']) || '客户'}、入库和出库。
              </Typography.Paragraph>
            )}
          </Card>
        )}
        {current === 5 && (
          <Card title="完成初始化">
            <Typography.Paragraph>
              初始化只写入商户信息、术语、行业模板、功能开关和默认单据设置，不会清空任何业务数据。
            </Typography.Paragraph>
            <Typography.Paragraph type="secondary">
              基础资料可稍后在系统设置中的“通用数据导入”区域导入；历史工作簿迁移位于系统设置的高级区域。
            </Typography.Paragraph>
          </Card>
        )}
      </Form>
      <Card>
        <Space>
          <Button disabled={current === 0} onClick={() => setCurrent((value) => Math.max(value - 1, 0))}>
            上一步
          </Button>
          {current < setupSteps.length - 1 ? (
            <Button type="primary" onClick={() => void nextStep()}>
              下一步
            </Button>
          ) : (
            <Button type="primary" onClick={() => void complete()}>
              完成初始化
            </Button>
          )}
        </Space>
      </Card>
    </div>
  );
}

function stepFields(step: number) {
  if (step === 0) {
    return [['merchant', 'name']];
  }
  if (step === 1) {
    return ['industryTemplate'];
  }
  if (step === 2) {
    return [
      ['terms', 'customer'],
      ['terms', 'region'],
      ['terms', 'product'],
      ['terms', 'category'],
      ['terms', 'rule'],
      ['terms', 'credit'],
      ['terms', 'guestCustomer']
    ];
  }
  return [];
}

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
