import { App, Button, DatePicker, Drawer, Input, Select, Space, Table, Tag, Typography } from 'antd';
import { useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { api } from '../api/inventory';
import PrintPreview from '../components/PrintPreview';
import type { DocumentDto, OrderDetailDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

export default function DocumentsPage() {
  const { message, modal } = App.useApp();
  const [params] = useSearchParams();
  const { customers, terms } = useAppStore();
  const [rows, setRows] = useState<DocumentDto[]>([]);
  const [customerId, setCustomerId] = useState<number | undefined>(() => {
    const value = params.get('customerId');
    return value ? Number(value) : undefined;
  });
  const [orderNo, setOrderNo] = useState('');
  const [range, setRange] = useState<[string, string]>();
  const [printed, setPrinted] = useState<boolean>();
  const [detail, setDetail] = useState<OrderDetailDto>();
  const [printers, setPrinters] = useState<string[]>([]);
  const [printerName, setPrinterName] = useState<string>();

  async function load() {
    try {
      setRows(await api.documents({
        customerId,
        orderNo,
        startDate: range?.[0],
        endDate: range?.[1],
        printed
      }));
    } catch (error) {
      message.error(error instanceof Error ? error.message : '单据加载失败');
    }
  }

  useEffect(() => {
    void load();
    void api.printers().then(setPrinters).catch(() => setPrinters([]));
  }, []);

  async function preview(row: DocumentDto) {
    try {
      setDetail(await api.order(row.orderId));
    } catch (error) {
      message.error(error instanceof Error ? error.message : '单据详情加载失败');
    }
  }

  async function reexport(row: DocumentDto) {
    try {
      const path = await api.exportDocument(row.orderId);
      message.success(`已重新导出：${path}`);
      await load();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '重新导出失败');
    }
  }

  async function exportPdf(row: DocumentDto) {
    try {
      const path = await api.exportDocumentPdf(row.orderId);
      message.success(`已导出 PDF：${path}`);
      await load();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导出 PDF 失败');
    }
  }

  async function print(row: DocumentDto) {
    try {
      const result = await api.printDocument(row.id, { printerName });
      message.success(result.message);
      await load();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '打印失败');
    }
  }

  async function voidOrder(row: DocumentDto) {
    modal.confirm({
      title: `作废订单 ${row.orderNo}？`,
      content: `作废会回滚该订单库存流水、${terms.credit}抵扣和单据状态。`,
      okText: '作废',
      okButtonProps: { danger: true },
      onOk: async () => {
        await api.voidOrder(row.orderId, { reason: '单据档案作废' });
        message.success('订单已作废');
        await load();
      }
    });
  }

  return (
    <div className="page">
      <div className="page-title"><Typography.Title level={2}>单据档案</Typography.Title></div>
      <div className="toolbar panel">
        <Select allowClear showSearch optionFilterProp="label" placeholder={terms.customer} value={customerId} style={{ width: 220 }} options={customers.map((item) => ({ value: item.id, label: item.name }))} onChange={setCustomerId} />
        <Input allowClear placeholder="单号" value={orderNo} onChange={(event) => setOrderNo(event.target.value)} style={{ width: 180 }} />
        <DatePicker.RangePicker onChange={(values) => setRange(values ? [values[0]!.format('YYYY-MM-DD'), values[1]!.format('YYYY-MM-DD')] : undefined)} />
        <Select allowClear placeholder="打印状态" style={{ width: 130 }} options={[{ value: true, label: '已打印' }, { value: false, label: '未打印' }]} onChange={setPrinted} />
        <Select allowClear placeholder="打印机" style={{ width: 220 }} options={printers.map((item) => ({ value: item, label: item }))} onChange={setPrinterName} />
        <Button onClick={() => void load()}>查询</Button>
      </div>
      <Table
        rowKey="id"
        dataSource={rows}
        columns={[
          { title: '单号', dataIndex: 'orderNo' },
          { title: terms.customer, dataIndex: 'customerName' },
          { title: '文件路径', dataIndex: 'filePath', ellipsis: true },
          { title: '打印次数', dataIndex: 'printCount', width: 100 },
          { title: '状态', render: (_, row) => <Tag color={row.status === 'voided' ? 'red' : 'green'}>{row.status === 'voided' ? '作废' : '正常'}</Tag>, width: 90 },
          { title: '创建时间', dataIndex: 'createdAt', width: 170 },
          {
            title: '操作',
            width: 380,
            render: (_, row) => (
              <Space>
                <Button size="small" onClick={() => void preview(row)}>预览</Button>
                <Button size="small" onClick={() => void api.openDocument(row.id)}>打开</Button>
                <Button size="small" onClick={() => void reexport(row)}>重新导出</Button>
                <Button size="small" onClick={() => void exportPdf(row)}>导出 PDF</Button>
                <Button size="small" onClick={() => void print(row)}>打印</Button>
                <Button size="small" danger disabled={row.status === 'voided'} onClick={() => void voidOrder(row)}>作废</Button>
              </Space>
            )
          }
        ]}
      />
      <Drawer
        title={detail ? `单据预览：${detail.order.orderNo}` : '单据预览'}
        open={!!detail}
        onClose={() => setDetail(undefined)}
        width={860}
      >
        <PrintPreview detail={detail} />
      </Drawer>
    </div>
  );
}
