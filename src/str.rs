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
    UnexpectedNodeType(&'static str),
    UnexpectedObjectType(ObjectType),
    Unreachable,
    Unsupported(String),
}

impl fmt::Display for SqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqlError::Missing(field) => write!(f, "Missing field: {}", field),
            SqlError::UnexpectedNodeType(node) => write!(f, "Unexpected node type: {}", node),
            SqlError::UnexpectedObjectType(ty) => write!(f, "Unexpected object type: {:?}", ty),
            SqlError::Unreachable => write!(f, "Unreachable"),
            SqlError::Unsupported(message) => write!(f, "Unsupported feature: {}", message),
        }
    }
}

impl std::error::Error for SqlError {}

trait SqlBuilder {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError>;
}

trait SqlBuilderWithContext {
    fn build_with_context(
        &self,
        buffer: &mut String,
        context: Context,
    ) -> core::result::Result<(), SqlError>;
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = String::new();
        match self.build(&mut buffer) {
            Ok(_) => write!(f, "{}", buffer),
            Err(_) => Err(fmt::Error),
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
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match self {
            Node::A_ArrayExpr(a_array_expr) => a_array_expr.build(buffer)?,
            Node::A_Const(a_const) => a_const.build(buffer)?,
            Node::A_Expr(a_expr) => a_expr.build_with_context(buffer, Context::None)?,
            Node::A_Indices(a_indices) => a_indices.build(buffer)?,
            Node::A_Indirection(a_indirection) => a_indirection.build(buffer)?,
            Node::A_Star(a_star) => a_star.build(buffer)?,
            Node::AccessPriv(privilege) => privilege.build(buffer)?,
            Node::AlterCollationStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterDatabaseSetStmt(stmt) => stmt.build(buffer)?,
            Node::AlterDatabaseStmt(stmt) => stmt.build(buffer)?,
            Node::AlterDefaultPrivilegesStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterDomainStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterEnumStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterEventTrigStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterExtensionContentsStmt(stmt) => stmt.build(buffer)?,
            Node::AlterExtensionStmt(stmt) => stmt.build(buffer)?,
            Node::AlterFdwStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterForeignServerStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterFunctionStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterObjectDependsStmt(stmt) => stmt.build(buffer)?,
            Node::AlterObjectSchemaStmt(stmt) => stmt.build(buffer)?,
            Node::AlterOpFamilyStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterOperatorStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterOwnerStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterPolicyStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterPublicationStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterRoleSetStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterRoleStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterSeqStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterStatsStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterSubscriptionStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterSystemStmt(stmt) => stmt.build(buffer)?,
            Node::AlterTSConfigurationStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterTSDictionaryStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterTableCmd(cmd) => cmd.build_with_context(buffer, Context::None)?,
            Node::AlterTableMoveAllStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterTableSpaceOptionsStmt(stmt) => stmt.build(buffer)?,
            Node::AlterTableStmt(stmt) => stmt.build(buffer)?,
            Node::AlterTypeStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::AlterUserMappingStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CallContext(context) => unimplemented!("{}: {:?}", self, context),
            Node::CallStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CheckPointStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::ClosePortalStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::ClusterStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CollateClause(collate) => collate.build(buffer)?,
            Node::ColumnDef(column) => column.build(buffer)?,
            Node::ColumnRef(column) => column.build(buffer)?,
            Node::CommentStmt(stmt) => stmt.build(buffer)?,
            Node::CommonTableExpr(cte) => cte.build(buffer)?,
            Node::CompositeTypeStmt(stmt) => stmt.build(buffer)?,
            Node::Constraint(constraint) => constraint.build(buffer)?,
            Node::ConstraintsSetStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CopyStmt(stmt) => stmt.build(buffer)?,
            Node::CreateAmStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateCastStmt(stmt) => stmt.build(buffer)?,
            Node::CreateConversionStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateDomainStmt(stmt) => stmt.build(buffer)?,
            Node::CreateEnumStmt(stmt) => stmt.build(buffer)?,
            Node::CreateEventTrigStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateExtensionStmt(stmt) => stmt.build(buffer)?,
            Node::CreateFdwStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateForeignServerStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateForeignTableStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateFunctionStmt(stmt) => stmt.build(buffer)?,
            Node::CreateOpClassItem(item) => unimplemented!("{}: {:?}", self, item),
            Node::CreateOpClassStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateOpFamilyStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreatePLangStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreatePolicyStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreatePublicationStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateRangeStmt(stmt) => stmt.build(buffer)?,
            Node::CreateRoleStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateSchemaStmt(stmt) => stmt.build(buffer)?,
            Node::CreateSeqStmt(stmt) => stmt.build(buffer)?,
            Node::CreateStatsStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateStmt(stmt) => stmt.build_with_context(buffer, Context::None)?,
            Node::CreateSubscriptionStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateTableAsStmt(stmt) => stmt.build(buffer)?,
            Node::CreateTableSpaceStmt(stmt) => stmt.build(buffer)?,
            Node::CreateTransformStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreateTrigStmt(stmt) => stmt.build(buffer)?,
            Node::CreateUserMappingStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CreatedbStmt(stmt) => stmt.build(buffer)?,
            Node::DeallocateStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::DeclareCursorStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::DefElem(elem) => unimplemented!("{}: {:?}", self, elem),
            Node::DefineStmt(stmt) => stmt.build(buffer)?,
            Node::DeleteStmt(stmt) => stmt.build(buffer)?,
            Node::DiscardStmt(stmt) => stmt.build(buffer)?,
            Node::DoStmt(stmt) => stmt.build(buffer)?,
            Node::DropOwnedStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::DropRoleStmt(stmt) => stmt.build(buffer)?,
            Node::DropStmt(stmt) => stmt.build(buffer)?,
            Node::DropSubscriptionStmt(stmt) => stmt.build(buffer)?,
            Node::DropTableSpaceStmt(stmt) => stmt.build(buffer)?,
            Node::DropUserMappingStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::DropdbStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::ExecuteStmt(stmt) => stmt.build(buffer)?,
            Node::ExplainStmt(stmt) => stmt.build(buffer)?,
            Node::FetchStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::FuncCall(func) => func.build(buffer)?,
            Node::FunctionParameter(parameter) => parameter.build(buffer)?,
            Node::GrantRoleStmt(stmt) => stmt.build(buffer)?,
            Node::GrantStmt(stmt) => stmt.build(buffer)?,
            Node::GroupingSet(set) => set.build(buffer)?,
            Node::ImportForeignSchemaStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::IndexElem(elem) => elem.build(buffer)?,
            Node::IndexStmt(stmt) => stmt.build(buffer)?,
            Node::InferClause(clause) => clause.build(buffer)?,
            Node::InlineCodeBlock(block) => unimplemented!("{}: {:?}", self, block),
            Node::InsertStmt(stmt) => stmt.build(buffer)?,
            Node::ListenStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::LoadStmt(stmt) => stmt.build(buffer)?,
            Node::LockStmt(stmt) => stmt.build(buffer)?,
            Node::LockingClause(clause) => clause.build(buffer)?,
            Node::MultiAssignRef(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::NotifyStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::ObjectWithArgs(args) => unimplemented!("{}: {:?}", self, args),
            Node::OnConflictClause(clause) => clause.build(buffer)?,
            Node::ParamRef(param) => param.build(buffer)?,
            Node::PartitionBoundSpec(bound) => bound.build(buffer)?,
            Node::PartitionCmd(cmd) => cmd.build(buffer)?,
            Node::PartitionElem(elem) => elem.build(buffer)?,
            Node::PartitionRangeDatum(datum) => unimplemented!("{}: {:?}", self, datum),
            Node::PartitionSpec(spec) => spec.build(buffer)?,
            Node::PrepareStmt(stmt) => stmt.build(buffer)?,
            Node::Query(query) => unimplemented!("{}: {:?}", self, query),
            Node::RangeFunction(func) => func.build(buffer)?,
            Node::RangeSubselect(select) => select.build(buffer)?,
            Node::RangeTableFunc(func) => func.build(buffer)?,
            Node::RangeTableFuncCol(col) => col.build(buffer)?,
            Node::RangeTableSample(sample) => sample.build(buffer)?,
            Node::RangeTblEntry(entry) => unimplemented!("{}: {:?}", self, entry),
            Node::RangeTblFunction(function) => unimplemented!("{}: {:?}", self, function),
            Node::RawStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::ReassignOwnedStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::RefreshMatViewStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::ReindexStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::RenameStmt(stmt) => stmt.build(buffer)?,
            Node::ReplicaIdentityStmt(stmt) => stmt.build(buffer)?,
            Node::ResTarget(target) => target.build(buffer)?,
            Node::RoleSpec(role) => role.build(buffer)?,
            Node::RowMarkClause(clause) => unimplemented!("{}: {:?}", self, clause),
            Node::RuleStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::SecLabelStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::SelectStmt(stmt) => stmt.build(buffer)?,
            Node::SetOperationStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::SortBy(sort) => sort.build(buffer)?,
            Node::SortGroupClause(clause) => unimplemented!("{}: {:?}", self, clause),
            Node::TableLikeClause(clause) => clause.build(buffer)?,
            Node::TableSampleClause(clause) => unimplemented!("{}: {:?}", self, clause),
            Node::TransactionStmt(stmt) => stmt.build(buffer)?,
            Node::TriggerTransition(transition) => transition.build(buffer)?,
            Node::TruncateStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::TypeCast(cast) => cast.build(buffer)?,
            Node::TypeName(name) => name.build(buffer)?,
            Node::UnlistenStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::UpdateStmt(stmt) => stmt.build(buffer)?,
            Node::VacuumRelation(relation) => unimplemented!("{}: {:?}", self, relation),
            Node::VacuumStmt(stmt) => stmt.build(buffer)?,
            Node::VariableSetStmt(stmt) => stmt.build(buffer)?,
            Node::VariableShowStmt(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::ViewStmt(stmt) => stmt.build(buffer)?,
            Node::WindowClause(clause) => unimplemented!("{}: {:?}", self, clause),
            Node::WindowDef(def) => def.build(buffer)?,
            Node::WithCheckOption(option) => unimplemented!("{}: {:?}", self, option),
            Node::WithClause(with) => with.build(buffer)?,
            Node::XmlSerialize(xml) => xml.build(buffer)?,
            Node::Aggref(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::Alias(alias) => alias.build(buffer)?,
            Node::AlternativeSubPlan(plan) => unimplemented!("{}: {:?}", self, plan),
            Node::ArrayCoerceExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::ArrayExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::BoolExpr(expr) => expr.build(buffer)?,
            Node::BooleanTest(test) => test.build(buffer)?,
            Node::CaseExpr(expr) => expr.build(buffer)?,
            Node::CaseTestExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::CaseWhen(when) => when.build(buffer)?,
            Node::CoalesceExpr(expr) => expr.build(buffer)?,
            Node::CoerceToDomain(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CoerceToDomainValue(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CoerceViaIO(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::CollateExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::Const(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::ConvertRowtypeExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::CurrentOfExpr(expr) => expr.build(buffer)?,
            Node::FieldSelect(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::FieldStore(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::FromExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::FuncExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::GroupingFunc(stmt) => stmt.build(buffer)?,
            Node::InferenceElem(stmt) => unimplemented!("{}: {:?}", self, stmt),
            Node::IntoClause(into) => into.build(buffer)?,
            Node::JoinExpr(expr) => expr.build(buffer)?,
            Node::MinMaxExpr(expr) => expr.build(buffer)?,
            Node::NamedArgExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::NextValueExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::NullTest(test) => test.build(buffer)?,
            Node::OnConflictExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::OpExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::Param(param) => unimplemented!("{}: {:?}", self, param),
            Node::RangeTblRef(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::RangeVar(range) => range.build_with_context(buffer, Context::None)?,
            Node::RelabelType(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::RowCompareExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::RowExpr(expr) => expr.build(buffer)?,
            Node::SQLValueFunction(func) => func.build(buffer)?,
            Node::ScalarArrayOpExpr(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::SetToDefault(set) => set.build(buffer)?,
            Node::SubLink(link) => link.build(buffer)?,
            Node::SubPlan(plan) => unimplemented!("{}: {:?}", self, plan),
            Node::SubscriptingRef(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::TableFunc(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::TargetEntry(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::Var(expr) => unimplemented!("{}: {:?}", self, expr),
            Node::WindowFunc(expr) => unimplemented!("{}: {:?}", self, expr),
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
            | Node::Float { .. }
            | Node::Integer { .. }
            | Node::Null { .. }
            | Node::String { .. } => SqlValue(self).build_with_context(buffer, Context::None)?,
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
        let node = Node::Integer { value: 5 };
        assert_eq!("Integer", node.name());
    }

    #[test]
    fn it_can_convert_a_empty_node_to_string() {
        let node = Node::Null {};
        assert_eq!("Null", node.name());
    }
}
