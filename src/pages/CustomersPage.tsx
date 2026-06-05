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
  const [batchForm] = Form.useForm();
  const { customers, terms, setCustomers } = useAppStore();
  const [region, setRegion] = useState<string>();
  const [keyword, setKeyword] = useState('');
  const [editing, setEditing] = useState<CustomerDto | null>(null);
  const [batchOpen, setBatchOpen] = useState(false);
  const [selectedCustomerIds, setSelectedCustomerIds] = useState<number[]>([]);
  const regions = useMemo(() => uniqueValues(customers, (item) => item.region), [customers]);
  const regionOptions = useMemo(() => regions.map((item) => ({ value: item, label: item })), [regions]);
  const isGuestCustomer = (row?: CustomerDto | null) => row?.name === terms.guestCustomer;
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

  function openBatchEditor() {
    if (selectedCustomerIds.length === 0) {
      message.warning(`请选择${terms.customer}`);
      return;
    }
    batchForm.resetFields();
    setBatchOpen(true);
  }

  async function saveBatch() {
    const values = await batchForm.validateFields();
    try {
      const result = await api.batchUpdateCustomers({
        ids: selectedCustomerIds,
        ...values
      });
      await refresh();
      setSelectedCustomerIds([]);
      setBatchOpen(false);
      message.success(`已批量更新 ${result.affectedCount} 个${terms.customer}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '批量编辑失败');
    }
  }

  async function disable(row: CustomerDto) {
    try {
      await api.disableCustomer(row.id);
      message.success(`${terms.customer}已删除`);
      await refresh();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '删除失败');
    }
  }

  return (
    <div className="page">
      <div className="page-title">
        <Typography.Title level={2}>{terms.customer}管理</Typography.Title>
        <Space>
          <Button disabled={selectedCustomerIds.length === 0} onClick={openBatchEditor}>批量编辑</Button>
          <Button type="primary" onClick={openNewCustomer}>新增{terms.customer}</Button>
        </Space>
      </div>
      <div className="toolbar panel">
        <Select allowClear placeholder={terms.region} value={region} style={{ width: 160 }} options={regionOptions} onChange={setRegion} />
        <Input allowClear placeholder={`搜索${terms.customer}或地址`} value={keyword} onChange={(event) => setKeyword(event.target.value)} style={{ width: 260 }} />
        <Button onClick={() => void refresh()}>刷新</Button>
      </div>
      <Table
        rowKey="id"
        dataSource={filtered}
        rowSelection={{
          selectedRowKeys: selectedCustomerIds,
          onChange: (keys) => setSelectedCustomerIds(keys as number[])
        }}
        columns={[
          { title: terms.region, dataIndex: 'region', width: 120 },
          { title: `${terms.customer}名称`, dataIndex: 'name' },
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
                  disabled={isGuestCustomer(row)}
                  onClick={() => modal.confirm({
                    title: `删除该${terms.customer}？`,
                    content: isGuestCustomer(row) ? `${terms.guestCustomer}是系统默认${terms.customer}，不能删除。` : `${terms.customer}会被停用并从常用列表隐藏，历史订单和单据不会被破坏。`,
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
      <Drawer title={editing?.id ? `编辑${terms.customer}` : `新增${terms.customer}`} open={!!editing} onClose={closeEditor} width={420}>
        <Form form={form} layout="vertical" className="dense-form">
          <Form.Item label={terms.region} name="region">
            <AutoComplete
              options={regionOptions}
              placeholder={`选择已有${terms.region}或输入新${terms.region}`}
              filterOption={(inputValue, option) => String(option?.value ?? '').toLowerCase().includes(inputValue.toLowerCase())}
            />
          </Form.Item>
          <Form.Item label={`${terms.customer}名称`} name="name" rules={[{ required: true, message: `请输入${terms.customer}名称` }]}><Input disabled={isGuestCustomer(editing)} /></Form.Item>
          <Form.Item label="地址" name="address"><Input /></Form.Item>
          <Form.Item label="联系方式" name="phone"><Input /></Form.Item>
          <Form.Item label="备注" name="remark"><Input.TextArea rows={3} /></Form.Item>
          <Button type="primary" block onClick={() => void save()}>保存</Button>
        </Form>
      </Drawer>
      <Drawer title={`批量编辑${terms.customer}（${selectedCustomerIds.length}）`} open={batchOpen} onClose={() => setBatchOpen(false)} width={420}>
        <Form form={batchForm} layout="vertical" className="dense-form">
          <Form.Item label={terms.region} name="region">
            <AutoComplete
              options={regionOptions}
              placeholder="留空则不修改"
              filterOption={(inputValue, option) => String(option?.value ?? '').toLowerCase().includes(inputValue.toLowerCase())}
            />
          </Form.Item>
          <Form.Item label="备注" name="remark"><Input.TextArea rows={3} /></Form.Item>
          <Form.Item label="状态" name="isActive">
            <Select
              allowClear
              placeholder="留空则不修改"
              options={[
                { value: true, label: '启用' },
                { value: false, label: '停用' }
              ]}
            />
          </Form.Item>
          <Button type="primary" block onClick={() => void saveBatch()}>保存批量修改</Button>
        </Form>
      </Drawer>
    </div>
  );
}
