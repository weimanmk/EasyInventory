import { App, Button, Card, DatePicker, Input, Select, Space, Typography } from 'antd';
import dayjs from 'dayjs';
import { useState } from 'react';
import { api } from '../api/inventory';
import { uniqueValues } from '../shared/format';
import { useAppStore } from '../store/appStore';

const exportTypes = [
  { value: 'products', label: '商品资料' },
  { value: 'customers', label: '客户资料' },
  { value: 'inbounds', label: '入库记录' },
  { value: 'monthly_credits', label: '月费账本' },
  { value: 'profits', label: '利润报表' },
  { value: 'inventory_report', label: '进销存报表' }
];

export default function ExportPage() {
  const { message } = App.useApp();
  const { customers, products } = useAppStore();
  const [exportType, setExportType] = useState('products');
  const [range, setRange] = useState<[string, string]>();
  const [customerId, setCustomerId] = useState<number>();
  const [category, setCategory] = useState<string>();
  const [status, setStatus] = useState<string>();
  const [keyword, setKeyword] = useState('');
  const [exporting, setExporting] = useState(false);
  const categories = uniqueValues(products, (item) => item.category);

  async function exportData(openAfter = false) {
    setExporting(true);
    try {
      const path = await api.exportData({
        exportType,
        startDate: range?.[0],
        endDate: range?.[1],
        customerId,
        category,
        status,
        keyword
      });
      message.success(`已导出：${path}`);
      if (openAfter) {
        await api.openExportsFolder();
      }
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导出失败');
    } finally {
      setExporting(false);
    }
  }

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>数据导出</Typography.Title>
          <Typography.Text type="secondary">导出常用业务表到 Excel</Typography.Text>
        </div>
      </div>
      <Card>
        <Space wrap size={12}>
          <Select style={{ width: 160 }} value={exportType} options={exportTypes} onChange={setExportType} />
          <DatePicker.RangePicker
            disabled={exportType === 'products' || exportType === 'customers'}
            defaultValue={[dayjs().startOf('month'), dayjs()]}
            onChange={(values) => setRange(values ? [values[0]!.format('YYYY-MM-DD'), values[1]!.format('YYYY-MM-DD')] : undefined)}
          />
          <Select
            allowClear
            showSearch
            optionFilterProp="label"
            placeholder="客户"
            disabled={exportType === 'products' || exportType === 'customers' || exportType === 'inbounds' || exportType === 'inventory_report'}
            style={{ width: 220 }}
            options={customers.map((item) => ({ value: item.id, label: item.name }))}
            onChange={setCustomerId}
          />
          <Select
            allowClear
            placeholder="类别"
            disabled={exportType === 'customers'}
            style={{ width: 160 }}
            options={categories.map((item) => ({ value: item, label: item }))}
            onChange={setCategory}
          />
          <Select
            allowClear
            placeholder="月费状态"
            disabled={exportType !== 'monthly_credits'}
            style={{ width: 140 }}
            options={[
              { value: 'pending', label: '未到期' },
              { value: 'available', label: '可用' },
              { value: 'used_up', label: '已用完' },
              { value: 'closed', label: '关闭' },
              { value: 'voided', label: '作废' }
            ]}
            onChange={setStatus}
          />
          <Input
            allowClear
            placeholder="关键字"
            disabled={!['products', 'customers', 'inventory_report'].includes(exportType)}
            value={keyword}
            onChange={(event) => setKeyword(event.target.value)}
            style={{ width: 220 }}
          />
          <Button type="primary" loading={exporting} onClick={() => void exportData(false)}>导出</Button>
          <Button loading={exporting} onClick={() => void exportData(true)}>导出并打开目录</Button>
        </Space>
      </Card>
    </div>
  );
}
