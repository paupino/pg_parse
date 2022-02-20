CREATE OR REPLACE VIEW account_transaction_with_balance AS
SELECT t.id, t.customer_id, t.account_id, t.transaction_date, t.fit_id, t.description, t.debit, t.credit, t.source, t.category,
       (COALESCE(sum_debit, 0::numeric) - COALESCE(sum_credit, 0::numeric)) + a.opening_balance "balance",
       r.reconciled_association_type,
       r.reconciled_association_id,
       r.reconciled_association_name,
       r.reconciled_transaction_desc
FROM
     (
         SELECT at.*,
                SUM(debit) OVER (PARTITION BY account_id ORDER BY transaction_date, id)  sum_debit,
                SUM(credit) OVER (PARTITION BY account_id ORDER BY transaction_date, id) sum_credit
         FROM account_transaction at
     ) t
     INNER JOIN account a ON a.id = t.account_id AND a.customer_id = t.customer_id
     LEFT JOIN
     (
         SELECT r.customer_id, r.source_transaction_id,
                -- Since 1 account transaction = multiple envelope in plan, we just use the plan
                CASE WHEN rs.id IS NOT NULL THEN 'plan' WHEN ra.id IS NOT NULL THEN 'account' WHEN re.id IS NOT NULL THEN 'envelope' END::text "reconciled_association_type",
                COALESCE(rs.id, ra.id, re.id) "reconciled_association_id",
                COALESCE(rs.name, COALESCE(ra.alias, ra.name), COALESCE(re.alias, re.name)) "reconciled_association_name",
                CASE WHEN rs.id IS NOT NULL THEN 'Allocated' ELSE COALESCE(rat.description, ret.description) END "reconciled_transaction_desc",
                ROW_NUMBER() OVER(
                    PARTITION BY r.customer_id, r.source_transaction_id
                    ) "rown"
         FROM public.account_reconciliation r
                  LEFT JOIN account_transaction rat on r.target_account_transaction_id = rat.id
                  LEFT JOIN account ra on rat.account_id = ra.id
                  LEFT JOIN envelope_transaction ret on r.target_envelope_transaction_id = ret.id
                  LEFT JOIN envelope re on ret.envelope_id = re.id
                  LEFT JOIN plan rs ON rs.id = r.plan_id
     ) r ON r.customer_id = t.customer_id AND r.source_transaction_id = t.id AND rown = 1
ORDER BY account_id, transaction_date DESC, id DESC;
