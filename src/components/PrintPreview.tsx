import { Descriptions, Table, Typography } from 'antd';
import { money, qty } from '../shared/format';
import type { OrderDetailDto } from '../shared/types';

type Props = {
  detail?: OrderDetailDto;
};

export default function PrintPreview({ detail }: Props) {
  if (!detail) {
    return <Typography.Text type="secondary">请选择单据</Typography.Text>;
  }

  const rows = detail.items.slice(0, 15);

  return (
    <div className="print-preview">
      <Typography.Title level={3} className="print-preview-title">科展商行</Typography.Title>
      <Descriptions bordered size="small" column={3} className="print-preview-meta">
        <Descriptions.Item label="客户">{detail.order.customerName}</Descriptions.Item>
        <Descriptions.Item label="单号">{detail.order.orderNo}</Descriptions.Item>
        <Descriptions.Item label="日期">{detail.order.orderDate}</Descriptions.Item>
        <Descriptions.Item label="地址" span={3}>{detail.order.customerAddress || '-'}</Descriptions.Item>
      </Descriptions>
      <Table
        rowKey="id"
        size="small"
        pagination={false}
        dataSource={rows}
        columns={[
          { title: '序号', width: 60, render: (_, __, index) => index + 1 },
          { title: '条码', dataIndex: 'barcode', width: 120 },
          { title: '商品名称', dataIndex: 'productName' },
          { title: '单位', width: 70, render: () => '件' },
          { title: '数量', width: 80, align: 'right', render: (_, row) => qty(row.quantity) },
          { title: '价格', width: 90, align: 'right', render: (_, row) => money(row.unitPrice) },
          { title: '总价格', width: 100, align: 'right', render: (_, row) => money(row.amount) },
          { title: '备注', dataIndex: 'remark', width: 140 }
        ]}
      />
      <div className="print-preview-total">
        <span>总金额：{money(detail.order.totals.customerPayableAmount)}</span>
        <span>总数量：{qty(rows.reduce((sum, item) => sum + item.quantity, 0))}</span>
        <span>明细金额：{money(rows.reduce((sum, item) => sum + item.amount, 0))}</span>
      </div>
      {detail.order.remark ? <Typography.Text type="secondary">备注：{detail.order.remark}</Typography.Text> : null}
    </div>
  );
}
