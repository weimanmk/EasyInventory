import { App, Button, Card, DatePicker, Input, Select, Space, Typography } from 'antd';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import { uniqueValues } from '../shared/format';
import { useAppStore } from '../store/appStore';

const productRankingOptions = [
  { value: 'sales_quantity', label: '销量' },
  { value: 'sales_amount', label: '销售额' },
  { value: 'profit_amount', label: '利润' },
  { value: 'gift_cost_amount', label: '赠品成本' }
];

const customerRankingOptions = [
  { value: 'sales_amount', label: '销售额' },
  { value: 'profit_amount', label: '利润' },
  { value: 'balance_amount', label: '欠款' }
];

const defaultExportRange = (): [string, string] => [
  dayjs().startOf('month').format('YYYY-MM-DD'),
  dayjs().format('YYYY-MM-DD')
];

export default function ExportPage() {
  const { message } = App.useApp();
  const { customers, products, terms, features } = useAppStore();
  const [exportType, setExportType] = useState('products');
  const [range, setRange] = useState<[string, string] | undefined>(defaultExportRange);
  const [customerId, setCustomerId] = useState<number>();
  const [category, setCategory] = useState<string>();
  const [status, setStatus] = useState<string>();
  const [rankBy, setRankBy] = useState('profit_amount');
  const [keyword, setKeyword] = useState('');
  const [exporting, setExporting] = useState(false);
  const categories = uniqueValues(products, (item) => item.category);
  const rankingOptions = exportType === 'customer_analysis' ? customerRankingOptions : productRankingOptions;
  const exportTypes = useMemo(() => [
    { value: 'products', label: `${terms.product}资料` },
    { value: 'customers', label: `${terms.customer}资料` },
    { value: 'inbounds', label: '入库记录' },
    ...(features.monthlyCredit ? [{ value: 'monthly_credits', label: `${terms.credit}账本` }] : []),
    { value: 'profits', label: '利润报表' },
    { value: 'inventory_report', label: '进销存报表' },
    ...(features.productRanking ? [{ value: 'product_ranking', label: `${terms.product}经营排行` }] : []),
    ...(features.customerAnalysis ? [{ value: 'customer_analysis', label: `${terms.customer}经营分析` }] : []),
    { value: 'customer_statement', label: `${terms.customer}对账单` }
  ], [features, terms]);

  useEffect(() => {
    if (!exportTypes.some((item) => item.value === exportType)) {
      setExportType('products');
    }
  }, [exportType, exportTypes]);

  async function exportData(openAfter = false) {
    if (exportType === 'customer_statement' && !customerId) {
      message.warning(`导出${terms.customer}对账单必须选择${terms.customer}`);
      return;
    }
    if (exportType === 'customer_statement' && !range) {
      message.warning(`导出${terms.customer}对账单必须选择日期范围`);
      return;
    }
    setExporting(true);
    try {
      const path = await api.exportData({
        exportType,
        startDate: range?.[0],
        endDate: range?.[1],
        customerId,
        category,
        rankBy,
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
          <Select
            style={{ width: 180 }}
            value={exportType}
            options={exportTypes}
            onChange={(value) => {
              setExportType(value);
              setRankBy(value === 'customer_analysis' ? 'sales_amount' : 'profit_amount');
            }}
          />
          <DatePicker.RangePicker
            disabled={exportType === 'products' || exportType === 'customers'}
            value={range ? [dayjs(range[0]), dayjs(range[1])] : undefined}
            onChange={(values) => setRange(values ? [values[0]!.format('YYYY-MM-DD'), values[1]!.format('YYYY-MM-DD')] : undefined)}
          />
          <Select
            allowClear
            showSearch
            optionFilterProp="label"
            placeholder={terms.customer}
            disabled={
              exportType === 'products' ||
              exportType === 'customers' ||
              exportType === 'inbounds' ||
              exportType === 'inventory_report' ||
              exportType === 'product_ranking' ||
              exportType === 'customer_analysis'
            }
            style={{ width: 220 }}
            options={customers.map((item) => ({ value: item.id, label: item.name }))}
            onChange={setCustomerId}
          />
          <Select
            allowClear
            placeholder={terms.category}
            disabled={exportType === 'customers' || exportType === 'customer_statement'}
            style={{ width: 160 }}
            options={categories.map((item) => ({ value: item, label: item }))}
            onChange={setCategory}
          />
          <Select
            allowClear
            placeholder={`${terms.credit}状态`}
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
          <Select
            placeholder="排行指标"
            disabled={!['product_ranking', 'customer_analysis'].includes(exportType)}
            value={rankBy}
            style={{ width: 140 }}
            options={rankingOptions}
            onChange={setRankBy}
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
