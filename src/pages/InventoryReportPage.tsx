import { App, Button, Card, DatePicker, Input, Select, Space, Statistic, Table, Typography } from 'antd';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import { money, qty, uniqueValues } from '../shared/format';
import type { InventoryReportRowDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

export default function InventoryReportPage() {
  const { message } = App.useApp();
  const { products, terms } = useAppStore();
  const [rows, setRows] = useState<InventoryReportRowDto[]>([]);
  const [range, setRange] = useState<[string, string]>([
    dayjs().startOf('month').format('YYYY-MM-DD'),
    dayjs().format('YYYY-MM-DD')
  ]);
  const [category, setCategory] = useState<string>();
  const [keyword, setKeyword] = useState('');
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const categories = useMemo(() => uniqueValues(products, (item) => item.category), [products]);
  const inboundAmount = rows.reduce((sum, row) => sum + row.inboundAmount, 0);
  const outboundAmount = rows.reduce((sum, row) => sum + row.outboundAmount, 0);
  const stockValue = rows.reduce((sum, row) => sum + row.stockValue, 0);
  const giftQuantity = rows.reduce((sum, row) => sum + row.giftQuantity, 0);

  async function loadRows() {
    setLoading(true);
    try {
      setRows(await api.inventoryReport({
        startDate: range[0],
        endDate: range[1],
        category,
        keyword
      }));
    } catch (error) {
      message.error(error instanceof Error ? error.message : '进销存报表加载失败');
    } finally {
      setLoading(false);
    }
  }

  async function exportReport(openAfter = false) {
    setExporting(true);
    try {
      const path = await api.exportData({
        exportType: 'inventory_report',
        startDate: range[0],
        endDate: range[1],
        category,
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

  useEffect(() => {
    void loadRows();
  }, []);

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>进销存报表</Typography.Title>
          <Typography.Text type="secondary">
            按日期、{terms.category}和关键字汇总入库、出库、赠品与当前库存
          </Typography.Text>
        </div>
        <Space>
          <Button loading={exporting} onClick={() => void exportReport(false)}>导出</Button>
          <Button type="primary" loading={loading} onClick={() => void loadRows()}>查询</Button>
        </Space>
      </div>
      <div className="toolbar panel">
        <DatePicker.RangePicker
          value={[dayjs(range[0]), dayjs(range[1])]}
          onChange={(values) => {
            if (values) {
              setRange([values[0]!.format('YYYY-MM-DD'), values[1]!.format('YYYY-MM-DD')]);
            }
          }}
        />
        <Select
          allowClear
          placeholder={terms.category}
          value={category}
          style={{ width: 180 }}
          options={categories.map((item) => ({ value: item, label: item }))}
          onChange={setCategory}
        />
        <Input
          allowClear
          placeholder={`${terms.product}名或条码`}
          value={keyword}
          onChange={(event) => setKeyword(event.target.value)}
          style={{ width: 240 }}
        />
        <Button onClick={() => void exportReport(true)} loading={exporting}>导出并打开目录</Button>
      </div>
      <div className="stat-grid">
        <Card><Statistic title="入库金额" value={money(inboundAmount)} /></Card>
        <Card><Statistic title="出库金额" value={money(outboundAmount)} /></Card>
        <Card><Statistic title="赠品数量" value={qty(giftQuantity)} /></Card>
        <Card><Statistic title="库存价值" value={money(stockValue)} /></Card>
        <Card><Statistic title={`${terms.product}行数`} value={rows.length} /></Card>
      </div>
      <Table
        rowKey="productId"
        loading={loading}
        dataSource={rows}
        scroll={{ x: 1180 }}
        columns={[
          { title: terms.product, dataIndex: 'productName', fixed: 'left', width: 220 },
          { title: terms.category, dataIndex: 'category', width: 120 },
          { title: '条码', dataIndex: 'barcode', width: 150 },
          { title: '入库数量', render: (_, row) => qty(row.inboundQuantity), align: 'right', width: 110 },
          { title: '入库金额', render: (_, row) => money(row.inboundAmount), align: 'right', width: 120 },
          { title: '出库数量', render: (_, row) => qty(row.outboundQuantity), align: 'right', width: 110 },
          { title: '出库金额', render: (_, row) => money(row.outboundAmount), align: 'right', width: 120 },
          { title: '赠品数量', render: (_, row) => qty(row.giftQuantity), align: 'right', width: 110 },
          { title: '当前库存', render: (_, row) => qty(row.currentStock), align: 'right', width: 110 },
          { title: '平均进货价', render: (_, row) => money(row.avgCost), align: 'right', width: 120 },
          { title: '库存价值', render: (_, row) => money(row.stockValue), align: 'right', width: 120 }
        ]}
      />
    </div>
  );
}
