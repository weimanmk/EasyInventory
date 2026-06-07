import type {
  BatchUpdateResultDto,
  CustomerDto,
  ProductDto,
  SupplierDto,
  SupplierPurchaseLedgerDto,
  SupplierPurchaseLedgerRequest
} from '../shared/types';
import { callCommand } from './tauri';

export const catalogApi = {
  products: (filter?: Record<string, unknown>) => callCommand<ProductDto[]>('list_products', { filter }),
  createProduct: (payload: Record<string, unknown>) => callCommand<ProductDto>('create_product', { payload }),
  updateProduct: (id: number, payload: Record<string, unknown>) => callCommand<ProductDto>('update_product', { id, payload }),
  disableProduct: (id: number) => callCommand<boolean>('disable_product', { id }),
  batchUpdateProducts: (payload: Record<string, unknown>) =>
    callCommand<BatchUpdateResultDto>('batch_update_products', { payload }),
  findProductByBarcode: (barcode: string) => callCommand<ProductDto | null>('find_product_by_barcode', { barcode }),
  customers: (filter?: Record<string, unknown>) => callCommand<CustomerDto[]>('list_customers', { filter }),
  createCustomer: (payload: Record<string, unknown>) => callCommand<CustomerDto>('create_customer', { payload }),
  updateCustomer: (id: number, payload: Record<string, unknown>) => callCommand<CustomerDto>('update_customer', { id, payload }),
  disableCustomer: (id: number) => callCommand<boolean>('disable_customer', { id }),
  batchUpdateCustomers: (payload: Record<string, unknown>) =>
    callCommand<BatchUpdateResultDto>('batch_update_customers', { payload }),
  suppliers: (filter?: Record<string, unknown>) => callCommand<SupplierDto[]>('list_suppliers', { filter }),
  createSupplier: (payload: Record<string, unknown>) => callCommand<SupplierDto>('create_supplier', { payload }),
  updateSupplier: (id: number, payload: Record<string, unknown>) => callCommand<SupplierDto>('update_supplier', { id, payload }),
  disableSupplier: (id: number) => callCommand<boolean>('disable_supplier', { id }),
  batchUpdateSuppliers: (payload: Record<string, unknown>) =>
    callCommand<BatchUpdateResultDto>('batch_update_suppliers', { payload }),
  supplierPurchaseLedger: (filter?: SupplierPurchaseLedgerRequest) =>
    callCommand<SupplierPurchaseLedgerDto>('get_supplier_purchase_ledger', { filter })
};
