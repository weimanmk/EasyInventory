import { App, AutoComplete, Button, Drawer, Form, Input, Select, Space, Table, Tag, Typography } from 'antd';
import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { api } from '../api/inventory';
import { writeClientLog } from '../api/tauri';
import { uniqueValues } from '../shared/format';
import type { CustomerDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

export default function CustomersPage() {
  const { message, modal } = App.useApp();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const { customers, setCustomers } = useAppStore();
  const [region, setRegion] = useState<string>();
  const [keyword, setKeyword] = useState('');
  const [editing, setEditing] = useState<CustomerDto | null>(null);
  const regions = useMemo(() => uniqueValues(customers, (item) => item.region), [customers]);
  const regionOptions = useMemo(() => regions.map((item) => ({ value: item, label: item })), [regions]);
  const filtered = customers.filter((item) => {
    const matchRegion = !region || item.region === region;
    const matchKeyword = !keyword || item.name.includes(keyword) || item.address?.includes(keyword);
    return matchRegion && matchKeyword;
  });

  async function refresh() {
    const nextCustomers = await api.customers({ isActive: true });
    setCustomers(nextCustomers);
    return nextCustomers;
  }

  async function save() {
    void writeClientLog('INFO', 'customers', '点击客户保存', { editingId: editing?.id ?? null });
    let values: Record<string, unknown>;
    try {
      values = await form.validateFields();
    } catch (error) {
      void writeClientLog('WARN', 'customers', '客户表单校验未通过', error);
      return;
    }
    try {
      let saved: CustomerDto;
      if (editing?.id) {
        void writeClientLog('INFO', 'customers', '提交更新客户', { id: editing.id, values });
        saved = await api.updateCustomer(editing.id, values);
      } else {
        void writeClientLog('INFO', 'customers', '提交新增客户', { values });
        saved = await api.createCustomer(values);
      }
      const nextCustomers = await refresh();
      const visibleCustomer = nextCustomers.find((item) => item.id === saved.id) ?? saved;
      void writeClientLog('INFO', 'customers', '客户保存后刷新完成', {
        savedId: saved.id,
        savedName: saved.name,
        savedRegion: saved.region,
        refreshedCount: nextCustomers.length,
        visibleAfterRefresh: nextCustomers.some((item) => item.id === saved.id)
      });
      setRegion(saved.region);
      setKeyword('');
      setEditing(null);
      form.resetFields();
      message.success(`保存成功：${visibleCustomer.name}`);
    } catch (error) {
      void writeClientLog('ERROR', 'customers', '客户保存失败', error);
      message.error(error instanceof Error ? error.message : '保存失败');
    }
  }

  function openNewCustomer() {
    form.resetFields();
    setEditing({} as CustomerDto);
  }

  function closeEditor() {
    setEditing(null);
    form.resetFields();
  }

  async function disable(row: CustomerDto) {
    try {
      await api.disableCustomer(row.id);
      message.success('客户已删除');
      await refresh();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '删除失败');
    }
  }

  return (
    <div className="page">
      <div className="page-title">
        <Typography.Title level={2}>客户管理</Typography.Title>
        <Button type="primary" onClick={openNewCustomer}>新增客户</Button>
      </div>
      <div className="toolbar panel">
        <Select allowClear placeholder="地区" value={region} style={{ width: 160 }} options={regionOptions} onChange={setRegion} />
        <Input allowClear placeholder="搜索客户或地址" value={keyword} onChange={(event) => setKeyword(event.target.value)} style={{ width: 260 }} />
        <Button onClick={() => void refresh()}>刷新</Button>
      </div>
      <Table
        rowKey="id"
        dataSource={filtered}
        columns={[
          { title: '地区', dataIndex: 'region', width: 120 },
          { title: '客户名称', dataIndex: 'name' },
          { title: '地址', dataIndex: 'address' },
          { title: '联系方式', dataIndex: 'phone', width: 140 },
          { title: '状态', render: (_, row) => <Tag color={row.isActive ? 'green' : 'default'}>{row.isActive ? '启用' : '停用'}</Tag>, width: 90 },
          {
            title: '操作',
            render: (_, row) => (
              <Space>
                <Button size="small" onClick={() => { form.resetFields(); setEditing(row); form.setFieldsValue(row); }}>编辑</Button>
                <Button size="small" onClick={() => navigate(`/documents?customerId=${row.id}`)}>历史单据</Button>
                <Button size="small" onClick={() => navigate(`/rules?customerId=${row.id}`)}>规则</Button>
                <Button
                  size="small"
                  danger
                  disabled={row.name === '散客'}
                  onClick={() => modal.confirm({
                    title: '删除该客户？',
                    content: row.name === '散客' ? '散客是系统默认客户，不能删除。' : '客户会被停用并从常用列表隐藏，历史订单和单据不会被破坏。',
                    okText: '删除',
                    okButtonProps: { danger: true },
                    onOk: () => disable(row)
                  })}
                >
                  删除
                </Button>
              </Space>
            ),
            width: 280
          }
        ]}
      />
      <Drawer title={editing?.id ? '编辑客户' : '新增客户'} open={!!editing} onClose={closeEditor} width={420}>
        <Form form={form} layout="vertical" className="dense-form">
          <Form.Item label="地区" name="region">
            <AutoComplete
              options={regionOptions}
              placeholder="选择已有地区或输入新地区"
              filterOption={(inputValue, option) => String(option?.value ?? '').toLowerCase().includes(inputValue.toLowerCase())}
            />
          </Form.Item>
          <Form.Item label="客户名称" name="name" rules={[{ required: true, message: '请输入客户名称' }]}><Input disabled={editing?.name === '散客'} /></Form.Item>
          <Form.Item label="地址" name="address"><Input /></Form.Item>
          <Form.Item label="联系方式" name="phone"><Input /></Form.Item>
          <Form.Item label="备注" name="remark"><Input.TextArea rows={3} /></Form.Item>
          <Button type="primary" block onClick={() => void save()}>保存</Button>
        </Form>
      </Drawer>
    </div>
  );
}
