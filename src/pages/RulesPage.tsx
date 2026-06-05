import { Alert, App, AutoComplete, Button, Drawer, Form, Input, InputNumber, Select, Space, Statistic, Table, Tag, Typography } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { api } from '../api/inventory';
import { writeClientLog } from '../api/tauri';
import { money, uniqueValues } from '../shared/format';
import type { CustomerProductRuleDto, CustomerProductRuleImportPreviewDto, CustomerProductRuleImportResultDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

export default function RulesPage() {
  const { message, modal } = App.useApp();
  const [params] = useSearchParams();
  const [form] = Form.useForm();
  const { customers, products, terms, features } = useAppStore();
  const [rules, setRules] = useState<CustomerProductRuleDto[]>([]);
  const [customerId, setCustomerId] = useState<number | undefined>(() => {
    const value = params.get('customerId');
    return value ? Number(value) : undefined;
  });
  const [category, setCategory] = useState<string>();
  const [editing, setEditing] = useState<CustomerProductRuleDto | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const [importPath, setImportPath] = useState('');
  const [importPreview, setImportPreview] = useState<CustomerProductRuleImportPreviewDto | null>(null);
  const [importResult, setImportResult] = useState<CustomerProductRuleImportResultDto | null>(null);
  const [importing, setImporting] = useState(false);
  const categories = useMemo(() => uniqueValues(products, (item) => item.category), [products]);
  const categoryOptions = useMemo(() => categories.map((item) => ({ value: item, label: item })), [categories]);
  const filteredProducts = products.filter((item) => !category || item.category === category);

  async function load() {
    if (!features.customerRules) {
      setRules([]);
      return;
    }
    setRules(await api.rules({ customerId, category, isActive: true }));
  }

  useEffect(() => {
    void load();
  }, [features.customerRules]);

  async function save() {
    void writeClientLog('INFO', 'rules', '点击规则保存', { editingId: editing?.id ?? null });
    let values: Record<string, unknown>;
    try {
      values = await form.validateFields();
    } catch (error) {
      void writeClientLog('WARN', 'rules', '规则表单校验未通过', error);
      return;
    }
    try {
      void writeClientLog('INFO', 'rules', editing?.id ? '提交更新规则' : '提交新增规则', {
        id: editing?.id ?? null,
        values
      });
      await api.saveRule({
        id: editing?.id,
        ...values,
        isActive: true
      });
      const product = products.find((item) => item.id === values.productId);
      const nextCategory = product?.category;
      const nextRules = await api.rules({ customerId: values.customerId, category: nextCategory, isActive: true });
      void writeClientLog('INFO', 'rules', '规则保存后刷新完成', {
        customerId: values.customerId,
        productId: values.productId,
        category: nextCategory,
        refreshedCount: nextRules.length
      });
      setCustomerId(values.customerId as number);
      setCategory(nextCategory);
      setEditing(null);
      form.resetFields();
      setRules(nextRules);
      message.success('规则已保存');
    } catch (error) {
      void writeClientLog('ERROR', 'rules', '规则保存失败', error);
      message.error(error instanceof Error ? error.message : '保存失败');
    }
  }

  function openNewRule() {
    form.resetFields();
    form.setFieldsValue({ customerId });
    setEditing({} as CustomerProductRuleDto);
  }

  function closeEditor() {
    setEditing(null);
    form.resetFields();
  }

  async function deleteRule(row: CustomerProductRuleDto) {
    try {
      await api.deleteRule(row.id);
      message.success('规则已删除');
      await load();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '删除失败');
    }
  }

  async function previewImport() {
    if (!importPath.trim()) {
      message.warning('请输入 Excel 文件路径');
      return;
    }
    setImporting(true);
    try {
      const preview = await api.previewRuleImport(importPath.trim());
      setImportPreview(preview);
      setImportResult(null);
      message.success(`预览完成：有效 ${preview.validCount} 行，异常 ${preview.errorCount} 行`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导入预览失败');
    } finally {
      setImporting(false);
    }
  }

  async function confirmImport() {
    if (!importPath.trim()) {
      message.warning('请输入 Excel 文件路径');
      return;
    }
    setImporting(true);
    try {
      const result = await api.importRules(importPath.trim());
      setImportResult(result);
      await load();
      message.success(`导入完成：成功 ${result.importedCount} 行，异常 ${result.errorCount} 行`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导入失败');
    } finally {
      setImporting(false);
    }
  }

  return (
    <div className="page">
      <div className="page-title">
        <Typography.Title level={2}>{terms.rule}</Typography.Title>
        {features.customerRules && (
          <Space>
            <Button onClick={() => setImportOpen(true)}>批量导入</Button>
            <Button type="primary" onClick={openNewRule}>新增{terms.rule}</Button>
          </Space>
        )}
      </div>
      {!features.customerRules && (
        <Alert
          type="info"
          showIcon
          message={`${terms.rule}功能已关闭`}
          description="可以在系统设置的功能开关中重新开启，历史规则数据会保留。"
        />
      )}
      {features.customerRules && (
        <>
      <div className="toolbar panel">
        <Select allowClear showSearch optionFilterProp="label" placeholder={terms.customer} value={customerId} style={{ width: 220 }} options={customers.map((item) => ({ value: item.id, label: item.name }))} onChange={setCustomerId} />
        <Select allowClear placeholder={terms.category} value={category} style={{ width: 160 }} options={categoryOptions} onChange={setCategory} />
        <Button onClick={() => void load()}>查询</Button>
      </div>
      <Table
        rowKey="id"
        dataSource={rules}
        columns={[
          { title: terms.customer, dataIndex: 'customerName' },
          { title: terms.product, dataIndex: 'productName' },
          { title: '固定售价', render: (_, row) => row.fixedPrice == null ? '-' : money(row.fixedPrice), align: 'right' },
          { title: '每满数量', dataIndex: 'thresholdQuantity', align: 'right' },
          { title: `赠品${terms.product}`, render: (_, row) => row.giftProductName ? `${row.giftProductName} x ${row.giftQuantity}` : '-' },
          { title: '折现', render: (_, row) => row.directDiscountAmount == null ? '-' : money(row.directDiscountAmount), align: 'right' },
          ...(features.monthlyCredit
            ? [{ title: terms.credit, render: (_: unknown, row: CustomerProductRuleDto) => row.monthlyCreditAmount == null ? '-' : money(row.monthlyCreditAmount), align: 'right' as const }]
            : []),
          { title: '状态', render: (_, row) => <Tag color={row.isActive ? 'green' : 'default'}>{row.isActive ? '启用' : '停用'}</Tag> },
          {
            title: '操作',
            render: (_, row) => (
              <Space>
                <Button size="small" onClick={() => { form.resetFields(); setEditing(row); form.setFieldsValue(row); }}>编辑</Button>
                <Button size="small" danger onClick={() => modal.confirm({ title: '停用该规则？', onOk: async () => { await api.disableRule(row.id); await load(); } })}>停用</Button>
                <Button
                  size="small"
                  danger
                  onClick={() => modal.confirm({
                    title: '永久删除该规则？',
                    content: '规则删除后不会再参与报价预览。历史订单中已记录的规则结果不会改变。',
                    okText: '删除',
                    okButtonProps: { danger: true },
                    onOk: () => deleteRule(row)
                  })}
                >
                  删除
                </Button>
              </Space>
            )
          }
        ]}
      />
      <Drawer title={editing?.id ? `编辑${terms.rule}` : `新增${terms.rule}`} open={!!editing} onClose={closeEditor} width={520}>
        <Form form={form} layout="vertical" className="dense-form">
          <Form.Item label={terms.customer} name="customerId" rules={[{ required: true, message: `请选择${terms.customer}` }]}>
            <Select showSearch optionFilterProp="label" options={customers.map((item) => ({ value: item.id, label: item.name }))} />
          </Form.Item>
          <Form.Item label={terms.product} name="productId" rules={[{ required: true, message: `请选择${terms.product}` }]}>
            <Select showSearch optionFilterProp="label" options={filteredProducts.map((item) => ({ value: item.id, label: `${item.category} / ${item.name}` }))} />
          </Form.Item>
          <Form.Item label="固定售价" name="fixedPrice"><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
          <Space.Compact block>
            <Form.Item label="每满数量" name="thresholdQuantity" style={{ width: '50%' }}><InputNumber min={0.01} style={{ width: '100%' }} /></Form.Item>
            <Form.Item label="直接折现" name="directDiscountAmount" style={{ width: '50%' }}><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
          </Space.Compact>
          <Space.Compact block>
            <Form.Item label={`赠品${terms.product}`} name="giftProductId" style={{ width: '70%' }}>
              <Select allowClear showSearch optionFilterProp="label" options={products.map((item) => ({ value: item.id, label: `${item.category} / ${item.name}` }))} />
            </Form.Item>
            <Form.Item label="赠品数量" name="giftQuantity" style={{ width: '30%' }}><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
          </Space.Compact>
          {features.monthlyCredit && (
            <Space.Compact block>
              <Form.Item label={`生成${terms.credit}`} name="monthlyCreditAmount" style={{ width: '50%' }}><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
              <Form.Item label={`${terms.credit}可用${terms.category}`} name="creditCategory" style={{ width: '50%' }}>
                <AutoComplete
                  options={categoryOptions}
                  placeholder={`选择已有${terms.category}或输入新${terms.category}`}
                  filterOption={(inputValue, option) => String(option?.value ?? '').toLowerCase().includes(inputValue.toLowerCase())}
                />
              </Form.Item>
            </Space.Compact>
          )}
          <Form.Item label="备注" name="remark"><Input.TextArea rows={3} /></Form.Item>
          <Button type="primary" block onClick={() => void save()}>保存{terms.rule}</Button>
        </Form>
      </Drawer>
      <Drawer title={`批量导入${terms.rule}`} open={importOpen} onClose={() => setImportOpen(false)} width={760}>
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Input
            value={importPath}
            onChange={(event) => setImportPath(event.target.value)}
            placeholder="Excel 文件路径，例如 C:/Users/ww/Desktop/work/客户商品规则导入.xlsx"
          />
          <Space>
            <Button loading={importing} onClick={() => void previewImport()}>预览</Button>
            <Button
              type="primary"
              loading={importing}
              disabled={!importPreview || importPreview.validCount === 0}
              onClick={() => modal.confirm({
                title: `确认导入${terms.rule}？`,
                content: `导入会停用同一${terms.customer}同一${terms.product}的旧活动规则，并插入新规则。异常行不会写入。`,
                okText: '导入',
                onOk: () => confirmImport()
              })}
            >
              确认导入
            </Button>
          </Space>
          <div className="stat-grid">
            <Statistic title="总行数" value={importResult?.rows.length ?? importPreview?.totalCount ?? 0} />
            <Statistic title="有效" value={importPreview?.validCount ?? 0} />
            <Statistic title="新增" value={importResult?.createCount ?? importPreview?.createCount ?? 0} />
            <Statistic title="覆盖" value={importResult?.overwriteCount ?? importPreview?.overwriteCount ?? 0} />
            <Statistic title="异常" value={importResult?.errorCount ?? importPreview?.errorCount ?? 0} />
            <Statistic title="跳过" value={importResult?.skippedCount ?? importPreview?.skippedCount ?? 0} />
          </div>
          <Table
            rowKey={(row) => `${row.rowNumber}-${row.customerName}-${row.productName}`}
            size="small"
            dataSource={importResult?.rows ?? importPreview?.rows ?? []}
            pagination={{ pageSize: 8 }}
            scroll={{ x: 980 }}
            columns={[
              { title: '行号', dataIndex: 'rowNumber', width: 70 },
              { title: terms.customer, dataIndex: 'customerName', width: 130 },
              { title: terms.product, dataIndex: 'productName', width: 150 },
              { title: terms.category, dataIndex: 'category', width: 100 },
              { title: '固定售价', render: (_, row) => row.fixedPrice == null ? '-' : money(row.fixedPrice), align: 'right', width: 100 },
              { title: '买赠', render: (_, row) => row.giftProductName ? `${row.thresholdQuantity ?? '-'} / ${row.giftProductName} x ${row.giftQuantity ?? '-'}` : '-', width: 180 },
              { title: '折现', render: (_, row) => row.directDiscountAmount == null ? '-' : money(row.directDiscountAmount), align: 'right', width: 100 },
              ...(features.monthlyCredit
                ? [{ title: terms.credit, render: (_: unknown, row: CustomerProductRuleImportPreviewDto['rows'][number]) => row.monthlyCreditAmount == null ? '-' : money(row.monthlyCreditAmount), align: 'right' as const, width: 100 }]
                : []),
              {
                title: '动作',
                render: (_, row) => <Tag color={row.action === 'overwrite' ? 'orange' : row.action === 'create' ? 'blue' : 'default'}>{actionText(row.action)}</Tag>,
                width: 90
              },
              {
                title: '状态',
                render: (_, row) => <Tag color={statusColor(row.status)}>{statusText(row.status)}</Tag>,
                width: 90
              },
              { title: '说明', dataIndex: 'message', width: 220 }
            ]}
          />
        </Space>
      </Drawer>
      </>
      )}
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
  return '跳过';
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
