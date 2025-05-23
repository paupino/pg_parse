#[macro_use]
mod helpers;
mod ext;
mod nodes;

use crate::ast::*;
use ext::*;
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Context {
    None,
    InsertRelation,
    AExpr,
    CreateType,
    AlterType,
    #[allow(dead_code)]
    Identifier,
    Constant,
    ForeignTable,
}

#[derive(Debug)]
enum SqlError {
    Missing(String),
    UnexpectedConstValue(&'static str),
    UnexpectedNodeType(&'static str),
    UnexpectedObjectType(ObjectType),
    Unreachable,
    Unsupported(String),
}

impl fmt::Display for SqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqlError::Missing(field) => write!(f, "Missing field: {}", field),
            SqlError::UnexpectedConstValue(value) => write!(f, "Unexpected const value: {}", value),
            SqlError::UnexpectedNodeType(node) => write!(f, "Unexpected node type: {}", node),
            SqlError::UnexpectedObjectType(ty) => write!(f, "Unexpected object type: {:?}", ty),
            SqlError::Unreachable => write!(f, "Unreachable"),
            SqlError::Unsupported(message) => write!(f, "Unsupported feature: {}", message),
        }
    }
}

impl std::error::Error for SqlError {}

trait SqlBuilder {
    fn build(&self, buffer: &mut String) -> Result<(), SqlError>;
}

trait SqlBuilderWithContext {
    fn build_with_context(&self, buffer: &mut String, context: Context) -> Result<(), SqlError>;
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = String::new();
        match self.build(&mut buffer) {
            Ok(_) => write!(f, "{}", buffer),
            Err(err) => {
                #[cfg(debug_assertions)]
                {
                    eprintln!("Error generating SQL: {}", err);
                }
                Err(fmt::Error)
            }
        }
    }
}

impl fmt::Display for BoolExprType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoolExprType::AND_EXPR => write!(f, "AND"),
            BoolExprType::OR_EXPR => write!(f, "OR"),
            BoolExprType::NOT_EXPR => write!(f, "NOT"),
        }
    }
}

impl SqlBuilder for Node {
    fn build(&self, buffer: &mut String) -> Result<(), SqlError> {
        match self {
            Node::A_ArrayExpr(a_array_expr) => a_array_expr.build(buffer)?,
            Node::A_Const(constant) => constant.build(buffer)?,
            Node::A_Expr(a_expr) => a_expr.build_with_context(buffer, Context::None)?,
            Node::A_Indices(a_indices) => a_indices.build(buffer)?,
            Node::A_Indirection(a_indirection) => a_indirection.build(buffer)?,
            Node::A_Star(a_star) => a_star.build(buffer)?,
            Node::AccessPriv(privilege) => privilege.build(buffer)?,
            Node::AlterCollationStmt(stmt) => unsupported!(stmt),
            Node::AlterDatabaseSetStmt(stmt) => stmt.build(buffer)?,
            Node::AlterDatabaseStmt(stmt) => stmt.build(buffer)?,
            Node::AlterDefaultPrivilegesStmt(stmt) => unsupported!(stmt),
            Node::AlterDomainStmt(stmt) => unsupported!(stmt),
            Node::AlterEnumStmt(stmt) => unsupported!(stmt),
            Node::AlterEventTrigStmt(stmt) => unsupported!(stmt),
            Node::AlterExtensionContentsStmt(stmt) => stmt.build(buffer)?,
            Node::AlterExtensionStmt(stmt) => stmt.build(buffer)?,
            Node::AlterFdwStmt(stmt) => unsupported!(stmt),
            Node::AlterForeignServerStmt(stmt) => unsupported!(stmt),
            Node::AlterFunctionStmt(stmt) => unsupported!(stmt),
            Node::AlterObjectDependsStmt(stmt) => stmt.build(buffer)?,
            Node::AlterObjectSchemaStmt(stmt) => stmt.build(buffer)?,
            Node::AlterOpFamilyStmt(stmt) => unsupported!(stmt),
            Node::AlterOperatorStmt(stmt) => unsupported!(stmt),
            Node::AlterOwnerStmt(stmt) => unsupported!(stmt),
            Node::AlterPolicyStmt(stmt) => unsupported!(stmt),
            Node::AlterPublicationStmt(stmt) => unsupported!(stmt),
            Node::AlterRoleSetStmt(stmt) => unsupported!(stmt),
            Node::AlterRoleStmt(stmt) => unsupported!(stmt),
            Node::AlterSeqStmt(stmt) => unsupported!(stmt),
            Node::AlterStatsStmt(stmt) => unsupported!(stmt),
            Node::AlterSubscriptionStmt(stmt) => unsupported!(stmt),
            Node::AlterSystemStmt(stmt) => stmt.build(buffer)?,
            Node::AlterTSConfigurationStmt(stmt) => unsupported!(stmt),
            Node::AlterTSDictionaryStmt(stmt) => unsupported!(stmt),
            Node::AlterTableCmd(cmd) => cmd.build_with_context(buffer, Context::None)?,
            Node::AlterTableMoveAllStmt(stmt) => unsupported!(stmt),
            Node::AlterTableSpaceOptionsStmt(stmt) => stmt.build(buffer)?,
            Node::AlterTableStmt(stmt) => stmt.build(buffer)?,
            Node::AlterTypeStmt(stmt) => unsupported!(stmt),
            Node::AlterUserMappingStmt(stmt) => unsupported!(stmt),
            Node::CallContext(context) => unsupported!(context),
            Node::CallStmt(stmt) => unsupported!(stmt),
            Node::CheckPointStmt(stmt) => unsupported!(stmt),
            Node::ClosePortalStmt(stmt) => unsupported!(stmt),
            Node::ClusterStmt(stmt) => unsupported!(stmt),
            Node::CollateClause(collate) => collate.build(buffer)?,
            Node::ColumnDef(column) => column.build(buffer)?,
            Node::ColumnRef(column) => column.build(buffer)?,
            Node::CommentStmt(stmt) => stmt.build(buffer)?,
            Node::CommonTableExpr(cte) => cte.build(buffer)?,
            Node::CompositeTypeStmt(stmt) => stmt.build(buffer)?,
            Node::Constraint(constraint) => constraint.build(buffer)?,
            Node::ConstraintsSetStmt(stmt) => unsupported!(stmt),
            Node::CopyStmt(stmt) => stmt.build(buffer)?,
            Node::CreateAmStmt(stmt) => unsupported!(stmt),
            Node::CreateCastStmt(stmt) => stmt.build(buffer)?,
            Node::CreateConversionStmt(stmt) => unsupported!(stmt),
            Node::CreateDomainStmt(stmt) => stmt.build(buffer)?,
            Node::CreateEnumStmt(stmt) => stmt.build(buffer)?,
            Node::CreateEventTrigStmt(stmt) => unsupported!(stmt),
            Node::CreateExtensionStmt(stmt) => stmt.build(buffer)?,
            Node::CreateFdwStmt(stmt) => unsupported!(stmt),
            Node::CreateForeignServerStmt(stmt) => unsupported!(stmt),
            Node::CreateForeignTableStmt(stmt) => unsupported!(stmt),
            Node::CreateFunctionStmt(stmt) => stmt.build(buffer)?,
            Node::CreateOpClassItem(item) => unsupported!(item),
            Node::CreateOpClassStmt(stmt) => unsupported!(stmt),
            Node::CreateOpFamilyStmt(stmt) => unsupported!(stmt),
            Node::CreatePLangStmt(stmt) => unsupported!(stmt),
            Node::CreatePolicyStmt(stmt) => unsupported!(stmt),
            Node::CreatePublicationStmt(stmt) => unsupported!(stmt),
            Node::CreateRangeStmt(stmt) => stmt.build(buffer)?,
            Node::CreateRoleStmt(stmt) => unsupported!(stmt),
            Node::CreateSchemaStmt(stmt) => stmt.build(buffer)?,
            Node::CreateSeqStmt(stmt) => stmt.build(buffer)?,
            Node::CreateStatsStmt(stmt) => unsupported!(stmt),
            Node::CreateStmt(stmt) => stmt.build_with_context(buffer, Context::None)?,
            Node::CreateSubscriptionStmt(stmt) => unsupported!(stmt),
            Node::CreateTableAsStmt(stmt) => stmt.build(buffer)?,
            Node::CreateTableSpaceStmt(stmt) => stmt.build(buffer)?,
            Node::CreateTransformStmt(stmt) => unsupported!(stmt),
            Node::CreateTrigStmt(stmt) => stmt.build(buffer)?,
            Node::CreateUserMappingStmt(stmt) => unsupported!(stmt),
            Node::CreatedbStmt(stmt) => stmt.build(buffer)?,
            Node::DeallocateStmt(stmt) => unsupported!(stmt),
            Node::DeclareCursorStmt(stmt) => unsupported!(stmt),
            Node::DefElem(elem) => unsupported!(elem),
            Node::DefineStmt(stmt) => stmt.build(buffer)?,
            Node::DeleteStmt(stmt) => stmt.build(buffer)?,
            Node::DiscardStmt(stmt) => stmt.build(buffer)?,
            Node::DoStmt(stmt) => stmt.build(buffer)?,
            Node::DropOwnedStmt(stmt) => unsupported!(stmt),
            Node::DropRoleStmt(stmt) => stmt.build(buffer)?,
            Node::DropStmt(stmt) => stmt.build(buffer)?,
            Node::DropSubscriptionStmt(stmt) => stmt.build(buffer)?,
            Node::DropTableSpaceStmt(stmt) => stmt.build(buffer)?,
            Node::DropUserMappingStmt(stmt) => unsupported!(stmt),
            Node::DropdbStmt(stmt) => unsupported!(stmt),
            Node::ExecuteStmt(stmt) => stmt.build(buffer)?,
            Node::ExplainStmt(stmt) => stmt.build(buffer)?,
            Node::FetchStmt(stmt) => unsupported!(stmt),
            Node::FuncCall(func) => func.build(buffer)?,
            Node::FunctionParameter(parameter) => parameter.build(buffer)?,
            Node::GrantRoleStmt(stmt) => stmt.build(buffer)?,
            Node::GrantStmt(stmt) => stmt.build(buffer)?,
            Node::GroupingSet(set) => set.build(buffer)?,
            Node::ImportForeignSchemaStmt(stmt) => unsupported!(stmt),
            Node::IndexElem(elem) => elem.build(buffer)?,
            Node::IndexStmt(stmt) => stmt.build(buffer)?,
            Node::InferClause(clause) => clause.build(buffer)?,
            Node::InlineCodeBlock(block) => unsupported!(block),
            Node::InsertStmt(stmt) => stmt.build(buffer)?,
            Node::ListenStmt(stmt) => unsupported!(stmt),
            Node::LoadStmt(stmt) => stmt.build(buffer)?,
            Node::LockStmt(stmt) => stmt.build(buffer)?,
            Node::LockingClause(clause) => clause.build(buffer)?,
            Node::MultiAssignRef(stmt) => unsupported!(stmt),
            Node::NotifyStmt(stmt) => unsupported!(stmt),
            Node::ObjectWithArgs(args) => unsupported!(args),
            Node::OnConflictClause(clause) => clause.build(buffer)?,
            Node::ParamRef(param) => param.build(buffer)?,
            Node::PartitionBoundSpec(bound) => bound.build(buffer)?,
            Node::PartitionCmd(cmd) => cmd.build(buffer)?,
            Node::PartitionElem(elem) => elem.build(buffer)?,
            Node::PartitionRangeDatum(datum) => unsupported!(datum),
            Node::PartitionSpec(spec) => spec.build(buffer)?,
            Node::PrepareStmt(stmt) => stmt.build(buffer)?,
            Node::Query(query) => unsupported!(query),
            Node::RangeFunction(func) => func.build(buffer)?,
            Node::RangeSubselect(select) => select.build(buffer)?,
            Node::RangeTableFunc(func) => func.build(buffer)?,
            Node::RangeTableFuncCol(col) => col.build(buffer)?,
            Node::RangeTableSample(sample) => sample.build(buffer)?,
            Node::RangeTblEntry(entry) => unsupported!(entry),
            Node::RangeTblFunction(function) => unsupported!(function),
            Node::RawStmt(stmt) => unsupported!(stmt),
            Node::ReassignOwnedStmt(stmt) => unsupported!(stmt),
            Node::RefreshMatViewStmt(stmt) => unsupported!(stmt),
            Node::ReindexStmt(stmt) => unsupported!(stmt),
            Node::RenameStmt(stmt) => stmt.build(buffer)?,
            Node::ReplicaIdentityStmt(stmt) => stmt.build(buffer)?,
            Node::ResTarget(target) => target.build(buffer)?,
            Node::RoleSpec(role) => role.build(buffer)?,
            Node::RowMarkClause(clause) => unsupported!(clause),
            Node::RuleStmt(stmt) => unsupported!(stmt),
            Node::SecLabelStmt(stmt) => unsupported!(stmt),
            Node::SelectStmt(stmt) => stmt.build(buffer)?,
            Node::SetOperationStmt(stmt) => unsupported!(stmt),
            Node::SortBy(sort) => sort.build(buffer)?,
            Node::SortGroupClause(clause) => unsupported!(clause),
            Node::TableLikeClause(clause) => clause.build(buffer)?,
            Node::TableSampleClause(clause) => unsupported!(clause),
            Node::TransactionStmt(stmt) => stmt.build(buffer)?,
            Node::TriggerTransition(transition) => transition.build(buffer)?,
            Node::TruncateStmt(stmt) => unsupported!(stmt),
            Node::TypeCast(cast) => cast.build(buffer)?,
            Node::TypeName(name) => name.build(buffer)?,
            Node::UnlistenStmt(stmt) => unsupported!(stmt),
            Node::UpdateStmt(stmt) => stmt.build(buffer)?,
            Node::VacuumRelation(relation) => unsupported!(relation),
            Node::VacuumStmt(stmt) => stmt.build(buffer)?,
            Node::VariableSetStmt(stmt) => stmt.build(buffer)?,
            Node::VariableShowStmt(stmt) => unsupported!(stmt),
            Node::ViewStmt(stmt) => stmt.build(buffer)?,
            Node::WindowClause(clause) => unsupported!(clause),
            Node::WindowDef(def) => def.build(buffer)?,
            Node::WithCheckOption(option) => unsupported!(option),
            Node::WithClause(with) => with.build(buffer)?,
            Node::XmlSerialize(xml) => xml.build(buffer)?,
            Node::Aggref(stmt) => unsupported!(stmt),
            Node::Alias(alias) => alias.build(buffer)?,
            Node::AlternativeSubPlan(plan) => unsupported!(plan),
            Node::ArrayCoerceExpr(expr) => unsupported!(expr),
            Node::ArrayExpr(expr) => unsupported!(expr),
            Node::BoolExpr(expr) => expr.build(buffer)?,
            Node::BooleanTest(test) => test.build(buffer)?,
            Node::CaseExpr(expr) => expr.build(buffer)?,
            Node::CaseTestExpr(expr) => unsupported!(expr),
            Node::CaseWhen(when) => when.build(buffer)?,
            Node::CoalesceExpr(expr) => expr.build(buffer)?,
            Node::CoerceToDomain(stmt) => unsupported!(stmt),
            Node::CoerceToDomainValue(stmt) => unsupported!(stmt),
            Node::CoerceViaIO(stmt) => unsupported!(stmt),
            Node::CollateExpr(expr) => unsupported!(expr),
            Node::Const(stmt) => unsupported!(stmt),
            Node::ConvertRowtypeExpr(expr) => unsupported!(expr),
            Node::CurrentOfExpr(expr) => expr.build(buffer)?,
            Node::FieldSelect(stmt) => unsupported!(stmt),
            Node::FieldStore(stmt) => unsupported!(stmt),
            Node::FromExpr(expr) => unsupported!(expr),
            Node::FuncExpr(expr) => unsupported!(expr),
            Node::GroupingFunc(stmt) => stmt.build(buffer)?,
            Node::InferenceElem(stmt) => unsupported!(stmt),
            Node::IntoClause(into) => into.build(buffer)?,
            Node::JoinExpr(expr) => expr.build(buffer)?,
            Node::MinMaxExpr(expr) => expr.build(buffer)?,
            Node::NamedArgExpr(expr) => unsupported!(expr),
            Node::NextValueExpr(expr) => unsupported!(expr),
            Node::NullTest(test) => test.build(buffer)?,
            Node::OnConflictExpr(expr) => unsupported!(expr),
            Node::OpExpr(expr) => unsupported!(expr),
            Node::Param(param) => unsupported!(param),
            Node::RangeTblRef(expr) => unsupported!(expr),
            Node::RangeVar(range) => range.build_with_context(buffer, Context::None)?,
            Node::RelabelType(expr) => unsupported!(expr),
            Node::RowCompareExpr(expr) => unsupported!(expr),
            Node::RowExpr(expr) => expr.build(buffer)?,
            Node::SQLValueFunction(func) => func.build(buffer)?,
            Node::ScalarArrayOpExpr(expr) => unsupported!(expr),
            Node::SetToDefault(set) => set.build(buffer)?,
            Node::SubLink(link) => link.build(buffer)?,
            Node::SubPlan(plan) => unsupported!(plan),
            Node::SubscriptingRef(expr) => unsupported!(expr),
            Node::TableFunc(expr) => unsupported!(expr),
            Node::TargetEntry(expr) => unsupported!(expr),
            Node::Var(expr) => unsupported!(expr),
            Node::WindowFunc(expr) => unsupported!(expr),
            Node::XmlExpr(expr) => expr.build(buffer)?,
            Node::List(list) => {
                for (index, item) in list.items.iter().enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    item.build(buffer)?;
                }
            }
            Node::BitString { .. }
            | Node::Boolean { .. }
            | Node::Float { .. }
            | Node::Integer { .. }
            | Node::String { .. } => SqlValue(self).build_with_context(buffer, Context::None)?,

            Node::AlterDatabaseRefreshCollStmt(stmt) => unsupported!(stmt),
            Node::CTECycleClause(clause) => unsupported!(clause),
            Node::CTESearchClause(clause) => unsupported!(clause),
            Node::MergeAction(action) => unsupported!(action),
            Node::MergeStmt(stmt) => unsupported!(stmt),
            Node::MergeWhenClause(clause) => unsupported!(clause),
            Node::PLAssignStmt(stmt) => unsupported!(stmt),
            Node::PublicationObjSpec(spec) => unsupported!(spec),
            Node::PublicationTable(table) => unsupported!(table),
            Node::ReturnStmt(stmt) => unsupported!(stmt),
            Node::StatsElem(elem) => unsupported!(elem),

            Node::JsonAggConstructor(arg) => unsupported!(arg),
            Node::JsonArgument(arg) => unsupported!(arg),
            Node::JsonArrayAgg(arg) => unsupported!(arg),
            Node::JsonArrayConstructor(arg) => unsupported!(arg),
            Node::JsonArrayQueryConstructor(arg) => unsupported!(arg),
            Node::JsonFuncExpr(arg) => unsupported!(arg),
            Node::JsonKeyValue(arg) => unsupported!(arg),
            Node::JsonObjectAgg(arg) => unsupported!(arg),
            Node::JsonObjectConstructor(arg) => unsupported!(arg),
            Node::JsonOutput(arg) => unsupported!(arg),
            Node::JsonParseExpr(arg) => unsupported!(arg),
            Node::JsonScalarExpr(arg) => unsupported!(arg),
            Node::JsonSerializeExpr(arg) => unsupported!(arg),
            Node::JsonTable(arg) => unsupported!(arg),
            Node::JsonTableColumn(arg) => unsupported!(arg),
            Node::JsonTablePathSpec(arg) => unsupported!(arg),
            Node::RTEPermissionInfo(arg) => unsupported!(arg),
            Node::SinglePartitionSpec(arg) => unsupported!(arg),
            Node::JsonBehavior(arg) => unsupported!(arg),
            Node::JsonConstructorExpr(arg) => unsupported!(arg),
            Node::JsonExpr(arg) => unsupported!(arg),
            Node::JsonFormat(arg) => unsupported!(arg),
            Node::JsonIsPredicate(arg) => unsupported!(arg),
            Node::JsonReturning(arg) => unsupported!(arg),
            Node::JsonTablePath(arg) => unsupported!(arg),
            Node::JsonTablePathScan(arg) => unsupported!(arg),
            Node::JsonTablePlan(arg) => unsupported!(arg),
            Node::JsonTableSiblingJoin(arg) => unsupported!(arg),
            Node::JsonValueExpr(arg) => unsupported!(arg),
            Node::MergeSupportFunc(arg) => unsupported!(arg),
            Node::WindowFuncRunCondition(arg) => unsupported!(arg),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{A_Star, Node};

    #[test]
    fn it_can_convert_a_struct_node_to_string() {
        let node = Node::A_Star(A_Star {});
        assert_eq!("A_Star", node.name());
    }

    #[test]
    fn it_can_convert_a_value_node_to_string() {
        let node = Node::Integer { ival: Some(5) };
        assert_eq!("Integer", node.name());
    }
}
