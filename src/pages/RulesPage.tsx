import { App, AutoComplete, Button, Drawer, Form, Input, InputNumber, Select, Space, Table, Tag, Typography } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { api } from '../api/inventory';
import { writeClientLog } from '../api/tauri';
import { money, uniqueValues } from '../shared/format';
import type { CustomerProductRuleDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

export default function RulesPage() {
  const { message, modal } = App.useApp();
  const [params] = useSearchParams();
  const [form] = Form.useForm();
  const { customers, products } = useAppStore();
  const [rules, setRules] = useState<CustomerProductRuleDto[]>([]);
  const [customerId, setCustomerId] = useState<number | undefined>(() => {
    const value = params.get('customerId');
    return value ? Number(value) : undefined;
  });
  const [category, setCategory] = useState<string>();
  const [editing, setEditing] = useState<CustomerProductRuleDto | null>(null);
  const categories = useMemo(() => uniqueValues(products, (item) => item.category), [products]);
  const categoryOptions = useMemo(() => categories.map((item) => ({ value: item, label: item })), [categories]);
  const filteredProducts = products.filter((item) => !category || item.category === category);

  async function load() {
    setRules(await api.rules({ customerId, category, isActive: true }));
  }

  useEffect(() => {
    void load();
  }, []);

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

  return (
    <div className="page">
      <div className="page-title">
        <Typography.Title level={2}>客户商品规则</Typography.Title>
        <Button type="primary" onClick={openNewRule}>新增规则</Button>
      </div>
      <div className="toolbar panel">
        <Select allowClear showSearch optionFilterProp="label" placeholder="客户" value={customerId} style={{ width: 220 }} options={customers.map((item) => ({ value: item.id, label: item.name }))} onChange={setCustomerId} />
        <Select allowClear placeholder="类别" value={category} style={{ width: 160 }} options={categoryOptions} onChange={setCategory} />
        <Button onClick={() => void load()}>查询</Button>
      </div>
      <Table
        rowKey="id"
        dataSource={rules}
        columns={[
          { title: '客户', dataIndex: 'customerName' },
          { title: '商品', dataIndex: 'productName' },
          { title: '固定售价', render: (_, row) => row.fixedPrice == null ? '-' : money(row.fixedPrice), align: 'right' },
          { title: '每满数量', dataIndex: 'thresholdQuantity', align: 'right' },
          { title: '赠品', render: (_, row) => row.giftProductName ? `${row.giftProductName} x ${row.giftQuantity}` : '-' },
          { title: '折现', render: (_, row) => row.directDiscountAmount == null ? '-' : money(row.directDiscountAmount), align: 'right' },
          { title: '月费', render: (_, row) => row.monthlyCreditAmount == null ? '-' : money(row.monthlyCreditAmount), align: 'right' },
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
      <Drawer title={editing?.id ? '编辑规则' : '新增规则'} open={!!editing} onClose={closeEditor} width={520}>
        <Form form={form} layout="vertical" className="dense-form">
          <Form.Item label="客户" name="customerId" rules={[{ required: true, message: '请选择客户' }]}>
            <Select showSearch optionFilterProp="label" options={customers.map((item) => ({ value: item.id, label: item.name }))} />
          </Form.Item>
          <Form.Item label="商品" name="productId" rules={[{ required: true, message: '请选择商品' }]}>
            <Select showSearch optionFilterProp="label" options={filteredProducts.map((item) => ({ value: item.id, label: `${item.category} / ${item.name}` }))} />
          </Form.Item>
          <Form.Item label="固定售价" name="fixedPrice"><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
          <Space.Compact block>
            <Form.Item label="每满数量" name="thresholdQuantity" style={{ width: '50%' }}><InputNumber min={0.01} style={{ width: '100%' }} /></Form.Item>
            <Form.Item label="直接折现" name="directDiscountAmount" style={{ width: '50%' }}><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
          </Space.Compact>
          <Space.Compact block>
            <Form.Item label="赠品商品" name="giftProductId" style={{ width: '70%' }}>
              <Select allowClear showSearch optionFilterProp="label" options={products.map((item) => ({ value: item.id, label: `${item.category} / ${item.name}` }))} />
            </Form.Item>
            <Form.Item label="赠品数量" name="giftQuantity" style={{ width: '30%' }}><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
          </Space.Compact>
          <Space.Compact block>
            <Form.Item label="生成月费" name="monthlyCreditAmount" style={{ width: '50%' }}><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
            <Form.Item label="月费可用类别" name="creditCategory" style={{ width: '50%' }}>
              <AutoComplete
                options={categoryOptions}
                placeholder="选择已有类别或输入新类别"
                filterOption={(inputValue, option) => String(option?.value ?? '').toLowerCase().includes(inputValue.toLowerCase())}
              />
            </Form.Item>
          </Space.Compact>
          <Form.Item label="备注" name="remark"><Input.TextArea rows={3} /></Form.Item>
          <Button type="primary" block onClick={() => void save()}>保存规则</Button>
        </Form>
      </Drawer>
    </div>
  );
}
