use crate::ast::{
    constants, DefElem, DefElemAction, DropBehavior, Node, ObjectWithArgs, ResTarget,
};
use crate::str::helpers::{
    is_keyword, is_operator, join_strings, node_vec_to_string_vec, non_reserved_word_or_sconst,
    quote_identifier,
};
use crate::str::{Context, SqlBuilder, SqlBuilderWithContext, SqlError};

pub(in crate::str) struct SqlValue<'a>(pub &'a Node);
impl SqlBuilderWithContext for SqlValue<'_> {
    fn build_with_context(
        &self,
        buffer: &mut String,
        context: Context,
    ) -> core::result::Result<(), SqlError> {
        match self.0 {
            Node::Integer { value } => buffer.push_str(&format!("{}", *value)),
            Node::Float { value } => {
                if let Some(value) = value {
                    buffer.push_str(value);
                }
            }
            Node::String { value } => {
                if let Some(value) = value {
                    match context {
                        Context::Identifier => buffer.push_str(&quote_identifier(value)),
                        Context::Constant => StringLiteral(value).build(buffer)?,
                        _ => buffer.push_str(value),
                    }
                }
            }
            Node::BitString { value } => {
                if let Some(value) = value {
                    let mut chars = value.chars();
                    if let Some(c) = chars.next() {
                        match c {
                            'x' => {
                                buffer.push('x');
                                StringLiteral(chars.as_str()).build(buffer)?;
                            }
                            'b' => {
                                buffer.push('b');
                                StringLiteral(chars.as_str()).build(buffer)?;
                            }
                            unknown => {
                                return Err(SqlError::Unsupported(format!(
                                    "Unknown bitstring modifier: {}",
                                    unknown
                                )))
                            }
                        }
                    } else {
                        return Err(SqlError::Unsupported("Empty bitstring".into()));
                    }
                } else {
                    return Err(SqlError::Unsupported("Empty bitstring".into()));
                }
            }
            Node::Null {} => buffer.push_str("NULL"),
            unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
        }
        Ok(())
    }
}

pub(in crate::str) struct Collate<'a>(pub &'a Vec<Node>);
impl SqlBuilder for Collate<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("COLLATE ");
        AnyName(self.0).build(buffer)
    }
}

pub(in crate::str) struct RelOptions<'a>(pub &'a Vec<Node>);
impl SqlBuilder for RelOptions<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push('(');
        for (index, elem) in iter_only!(self.0, Node::DefElem).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            if let Some(ref namespace) = elem.defnamespace {
                buffer.push_str(&quote_identifier(namespace));
                buffer.push('.');
            }
            if let Some(ref name) = elem.defname {
                buffer.push_str(&quote_identifier(name));
                if elem.arg.is_some() {
                    buffer.push('=');
                }
            }
            if let Some(ref arg) = elem.arg {
                DefArg(&**arg, false).build(buffer)?;
            }
        }
        buffer.push(')');
        Ok(())
    }
}

pub(in crate::str) struct DefArg<'a>(pub &'a Node, bool);
impl SqlBuilder for DefArg<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match self.0 {
            Node::TypeName(type_name) => type_name.build(buffer)?,
            Node::List(list) => match list.items.len() {
                1 => {
                    buffer.push_str(string_value!(list.items[0]));
                }
                2 => {
                    buffer.push_str("OPERATOR(");
                    AnyOperator(&list.items).build(buffer)?;
                    buffer.push(')');
                }
                _ => {
                    return Err(SqlError::Unsupported(
                        "Unexpected number of elements".into(),
                    ))
                }
            },
            Node::Float { .. } | Node::Integer { .. } => {
                SqlValue(self.0).build_with_context(buffer, Context::None)?
            }
            Node::String { value: Some(value) } => {
                if !self.1 && value.eq("none") {
                    buffer.push_str("NONE");
                } else if is_keyword(value) {
                    buffer.push_str(value);
                } else {
                    StringLiteral(value).build(buffer)?;
                }
            }
            unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
        }

        Ok(())
    }
}

pub(in crate::str) struct FromClause<'a>(pub &'a Vec<Node>);
impl SqlBuilder for FromClause<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("FROM ");
        FromList(self.0).build(buffer)
    }
}

pub(in crate::str) struct WhereClause<'a>(pub &'a Node);
impl SqlBuilder for WhereClause<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("WHERE ");
        Expr(self.0).build(buffer)
    }
}

pub(in crate::str) struct ExprList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for ExprList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, node) in self.0.iter().enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            Expr(node).build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct InsertColumnList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for InsertColumnList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, tgt) in iter_only!(self.0, Node::ResTarget).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }

            // Start with the name
            buffer.push_str(&quote_identifier(must!(tgt.name)));
            if let Some(ref indirection) = tgt.indirection {
                Indirection(indirection, 0).build(buffer)?;
            }
        }
        Ok(())
    }
}

pub(in crate::str) struct CreateGenericOptions<'a>(pub &'a Vec<Node>);
impl SqlBuilder for CreateGenericOptions<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("OPTIONS (");
        for (index, elem) in iter_only!(self.0, Node::DefElem).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            let name = must!(elem.defname);
            let arg = must!(elem.arg);
            let arg = string_value!(**arg);
            buffer.push_str(&quote_identifier(name));
            buffer.push(' ');
            StringLiteral(arg).build(buffer)?;
        }
        buffer.push(')');
        Ok(())
    }
}

pub(in crate::str) struct SetClauseList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for SetClauseList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        // Assert there is something to set
        let target_list = iter_only!(self.0, Node::ResTarget).collect::<Vec<_>>();
        if target_list.is_empty() {
            return Err(SqlError::Unsupported("Empty set clause".into()));
        }

        // Variable to keep track of elements to skip
        let mut skip = 0;
        for (index, target) in target_list.iter().enumerate() {
            if skip > 0 {
                skip -= 1;
                continue;
            }

            if index > 0 {
                buffer.push_str(", ");
            }

            let val = must!(target.val);

            if let Node::MultiAssignRef(ref mar) = **val {
                buffer.push('(');
                for (inner_index, inner_target) in target_list.iter().enumerate() {
                    SetTarget(*inner_target).build(buffer)?;
                    if inner_index as i32 == mar.ncolumns - 1 {
                        break;
                    } else if inner_index < target_list.len() - 1 {
                        buffer.push_str(", ");
                    }
                }
                buffer.push_str(") = ");
                if let Some(ref source) = mar.source {
                    Expr(&**source).build(buffer)?;
                }
                skip = mar.ncolumns - 1;
            } else {
                SetTarget(target).build(buffer)?;
                buffer.push_str(" = ");
                Expr(&**val).build(buffer)?;
            }
        }

        Ok(())
    }
}

pub(in crate::str) struct SetTarget<'a>(pub &'a ResTarget);
impl SqlBuilder for SetTarget<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.0.name);
        ColId(name).build(buffer)?;
        if let Some(ref indirection) = self.0.indirection {
            Indirection(indirection, 0).build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct TargetList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for TargetList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, tgt) in iter_only!(self.0, Node::ResTarget).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }

            let val = must!(tgt.val);
            match &**val {
                Node::ColumnRef(col) => col.build(buffer)?,
                other => Expr(other).build(buffer)?,
            }

            // Name
            if let Some(ref name) = tgt.name {
                buffer.push_str(" AS ");
                buffer.push_str(&quote_identifier(name));
            }
        }
        Ok(())
    }
}

pub(in crate::str) struct SortClause<'a>(pub &'a Vec<Node>);
impl SqlBuilder for SortClause<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if self.0.is_empty() {
            return Ok(());
        }
        buffer.push_str("ORDER BY ");
        for (index, node) in self.0.iter().enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }

            node!(node, Node::SortBy).build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct ParenthesizedSeqOptList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for ParenthesizedSeqOptList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if !self.0.is_empty() {
            buffer.push('(');
            SeqOptList(self.0).build(buffer)?;
            buffer.push(')');
        }
        Ok(())
    }
}

pub(in crate::str) struct FunctionWithArgTypes<'a>(pub &'a ObjectWithArgs);
impl SqlBuilder for FunctionWithArgTypes<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.0.objname);
        FuncName(name).build(buffer)?;
        if !self.0.args_unspecified {
            buffer.push('(');
            if let Some(ref args) = self.0.objargs {
                for (index, arg) in args.iter().enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    // Either a TypeName or FunctionParameter
                    match arg {
                        Node::TypeName(name) => name.build(buffer)?,
                        Node::FunctionParameter(parameter) => parameter.build(buffer)?,
                        unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
                    }
                }
            }
            buffer.push(')');
        }
        Ok(())
    }
}

pub(in crate::str) struct AggregateWithArgTypes<'a>(pub &'a ObjectWithArgs);
impl SqlBuilder for AggregateWithArgTypes<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.0.objname);
        FuncName(name).build(buffer)?;
        buffer.push('(');
        if let Some(ref args) = self.0.objargs {
            for (index, arg) in iter_only!(args, Node::TypeName).enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                }
                arg.build(buffer)?;
            }
        } else {
            buffer.push('*');
        }
        buffer.push(')');
        Ok(())
    }
}

pub(in crate::str) struct OperatorWithArgTypes<'a>(pub &'a ObjectWithArgs);
impl SqlBuilder for OperatorWithArgTypes<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.0.objname);
        AnyOperator(name).build(buffer)?;

        let args = must!(self.0.objargs);
        if args.len() != 2 {
            return Err(SqlError::Unsupported(format!(
                "Unsupported operator args: {}",
                args.len()
            )));
        }

        buffer.push('(');
        match &args[0] {
            Node::TypeName(typ) => typ.build(buffer)?,
            _ => buffer.push_str("NONE"),
        }
        buffer.push_str(", ");
        match &args[1] {
            Node::TypeName(typ) => typ.build(buffer)?,
            _ => buffer.push_str("NONE"),
        }
        buffer.push(')');
        Ok(())
    }
}

pub(in crate::str) struct SubqueryOperator<'a>(pub &'a Vec<Node>);
impl SqlBuilder for SubqueryOperator<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let list = self.0;
        if list.len() == 1 {
            if let Node::String {
                value: Some(ref value),
            } = list[0]
            {
                let handled = match &value[..] {
                    "~~" => {
                        buffer.push_str("LIKE");
                        true
                    }
                    "!~~" => {
                        buffer.push_str("NOT LIKE");
                        true
                    }
                    "~~*" => {
                        buffer.push_str("ILIKE");
                        true
                    }
                    "!~~*" => {
                        buffer.push_str("NOT ILIKE");
                        true
                    }
                    op => {
                        if is_operator(op) {
                            buffer.push_str(op);
                            true
                        } else {
                            false
                        }
                    }
                };

                // If it's handled then exit out
                if handled {
                    return Ok(());
                }
            }
        }

        buffer.push_str("OPERATOR(");
        AnyOperator(list).build(buffer)?;
        buffer.push(')');

        Ok(())
    }
}

pub(in crate::str) struct QualifiedOperator<'a>(pub &'a Vec<Node>);
impl SqlBuilder for QualifiedOperator<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let list = self.0;
        if list.len() == 1 {
            if let Node::String {
                value: Some(ref value),
            } = list[0]
            {
                if is_operator(value) {
                    buffer.push_str(value);
                    return Ok(());
                }
            }
        }

        buffer.push_str("OPERATOR(");
        AnyOperator(list).build(buffer)?;
        buffer.push(')');

        Ok(())
    }
}

pub(in crate::str) struct AnyOperator<'a>(pub &'a Vec<Node>);
impl SqlBuilder for AnyOperator<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let list = node_vec_to_string_vec(self.0);
        if list.is_empty() {
            return Err(SqlError::Missing("list".into()));
        }
        if list.len() > 2 {
            return Err(SqlError::Unsupported("list.len > 2".into()));
        }

        let mut iter = list.iter();
        if list.len() > 1 {
            buffer.push_str(&quote_identifier(iter.next().unwrap()));
            buffer.push('.');
        }
        buffer.push_str(iter.next().unwrap());

        Ok(())
    }
}

pub(in crate::str) struct FuncName<'a>(pub &'a Vec<Node>);
impl SqlBuilder for FuncName<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        join_strings(buffer, self.0, ".")
    }
}

pub(in crate::str) struct AnyName<'a>(pub &'a [Node]);
impl SqlBuilder for AnyName<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        join_strings(buffer, self.0, ".")
    }
}

pub(in crate::str) struct AnyNameList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for AnyNameList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, list) in iter_only!(self.0, Node::List).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            AnyName(&list.items).build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct ColumnList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for ColumnList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        join_strings(buffer, self.0, ", ")
    }
}

pub(in crate::str) struct FromList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for FromList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, table_ref) in self.0.iter().enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            TableRef(table_ref).build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct NameList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for NameList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, node) in self.0.iter().enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            let value = string_value!(node);
            ColId(value).build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct XmlAttributeList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for XmlAttributeList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, item) in iter_only!(self.0, Node::ResTarget).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }

            if let Some(ref val) = item.val {
                Expr(&**val).build(buffer)?;
            }
            if let Some(ref name) = item.name {
                buffer.push_str(" AS ");
                buffer.push_str(&quote_identifier(name));
            }
        }
        Ok(())
    }
}

pub(in crate::str) struct Indirection<'a>(pub &'a Vec<Node>, pub usize);
impl SqlBuilder for Indirection<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for node in self.0.iter().skip(self.1) {
            match node {
                Node::String { value } => {
                    // Column label
                    if let Some(value) = value {
                        buffer.push('.');
                        buffer.push_str(&quote_identifier(value));
                    }
                }
                Node::A_Star(_) => buffer.push_str(".*"),
                Node::A_Indices(indices) => indices.build(buffer)?,
                unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
            }
        }
        Ok(())
    }
}

pub(in crate::str) struct ColId<'a>(pub &'a str);
impl SqlBuilder for ColId<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str(&quote_identifier(self.0));
        Ok(())
    }
}

pub(in crate::str) struct StringLiteral<'a>(pub &'a str);
impl SqlBuilder for StringLiteral<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let val = self.0;
        if val.contains('\\') {
            buffer.push('E');
        }
        buffer.push('\'');
        for char in self.0.chars() {
            // "Escape" the ' char by doing a double
            if char == '\'' || char == '\\' {
                buffer.push(char);
            }
            buffer.push(char);
        }
        buffer.push('\'');
        Ok(())
    }
}

// "NonReservedWord_or_Sconst" in gram.y
//
// Note since both identifiers and string constants are allowed here, we
// currently always return an identifier, except:
//
// 1) when the string is empty (since an empty identifier can't be scanned)
// 2) when the value is equal or larger than NAMEDATALEN (64+ characters)
pub(in crate::str) struct NonReservedWordOrSconst<'a>(pub &'a Node);
impl SqlBuilder for NonReservedWordOrSconst<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let val = string_value!(self.0);
        non_reserved_word_or_sconst(buffer, val)
    }
}

pub(in crate::str) struct Expr<'a>(pub &'a Node);
impl SqlBuilder for Expr<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match self.0 {
            Node::FuncCall(inner) => inner.build(buffer)?,
            Node::XmlExpr(inner) => inner.build(buffer)?,
            Node::TypeCast(inner) => inner.build(buffer)?,
            Node::A_Const(inner) => inner.build(buffer)?,
            Node::ColumnRef(inner) => inner.build(buffer)?,
            Node::A_Expr(inner) => inner.build_with_context(buffer, Context::None)?,
            Node::CaseExpr(inner) => inner.build(buffer)?,
            Node::A_ArrayExpr(inner) => inner.build(buffer)?,
            Node::NullTest(inner) => inner.build(buffer)?,
            Node::XmlSerialize(inner) => inner.build(buffer)?,
            Node::ParamRef(inner) => inner.build(buffer)?,
            Node::BoolExpr(inner) => inner.build(buffer)?,
            Node::SubLink(inner) => inner.build(buffer)?,
            Node::RowExpr(inner) => inner.build(buffer)?,
            Node::CoalesceExpr(inner) => inner.build(buffer)?,
            Node::SetToDefault(inner) => inner.build(buffer)?,
            Node::A_Indirection(inner) => inner.build(buffer)?,
            Node::CollateClause(inner) => inner.build(buffer)?,
            Node::CurrentOfExpr(inner) => inner.build(buffer)?,
            Node::SQLValueFunction(inner) => inner.build(buffer)?,
            Node::MinMaxExpr(inner) => inner.build(buffer)?,
            Node::BooleanTest(inner) => inner.build(buffer)?,
            Node::GroupingFunc(inner) => inner.build(buffer)?,
            unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
        }
        Ok(())
    }
}

pub(in crate::str) struct TableRef<'a>(pub &'a Node);
impl SqlBuilder for TableRef<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match self.0 {
            Node::RangeVar(var) => var.build_with_context(buffer, Context::None)?,
            Node::RangeTableSample(sample) => sample.build(buffer)?,
            Node::RangeFunction(func) => func.build(buffer)?,
            Node::RangeTableFunc(func) => func.build(buffer)?,
            Node::RangeSubselect(subselect) => subselect.build(buffer)?,
            Node::JoinExpr(expr) => expr.build(buffer)?,
            unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
        }
        Ok(())
    }
}

pub(in crate::str) struct FuncExprWindowless<'a>(pub &'a Node);
impl SqlBuilder for FuncExprWindowless<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match self.0 {
            Node::FuncCall(func) => func.build(buffer)?,
            Node::SQLValueFunction(func) => func.build(buffer)?,
            Node::TypeCast(cast) => cast.build(buffer)?,
            Node::CoalesceExpr(expr) => expr.build(buffer)?,
            Node::MinMaxExpr(expr) => expr.build(buffer)?,
            Node::XmlExpr(expr) => expr.build(buffer)?,
            Node::XmlSerialize(serialize) => serialize.build(buffer)?,
            unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
        }
        Ok(())
    }
}

pub(in crate::str) struct PreparableStmt<'a>(pub &'a Node);
impl SqlBuilder for PreparableStmt<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match self.0 {
            Node::SelectStmt(stmt) => stmt.build(buffer)?,
            Node::InsertStmt(stmt) => stmt.build(buffer)?,
            Node::UpdateStmt(stmt) => stmt.build(buffer)?,
            Node::DeleteStmt(stmt) => stmt.build(buffer)?,
            unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
        }
        Ok(())
    }
}

pub(in crate::str) struct SchemaStmt<'a>(pub &'a Node);
impl SqlBuilder for SchemaStmt<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match self.0 {
            Node::CreateStmt(stmt) => stmt.build_with_context(buffer, Context::None)?,
            Node::IndexStmt(stmt) => stmt.build(buffer)?,
            Node::CreateSeqStmt(stmt) => stmt.build(buffer)?,
            Node::CreateTrigStmt(stmt) => stmt.build(buffer)?,
            Node::GrantStmt(stmt) => stmt.build(buffer)?,
            Node::ViewStmt(stmt) => stmt.build(buffer)?,
            unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
        }
        Ok(())
    }
}

pub(in crate::str) struct CommonFuncOptItem<'a>(pub &'a DefElem);
impl SqlBuilder for CommonFuncOptItem<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.0.defname);
        let arg = must!(self.0.arg);
        if name.eq("strict") {
            match &**arg {
                Node::Integer { value: 0 } => buffer.push_str("CALLED ON NULL INPUT"),
                Node::Integer { value: 1 } => buffer.push_str("RETURNS NULL ON NULL INPUT"),
                unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
            }
        } else if name.eq("volatility") {
            let volatility = string_value!(**arg);
            match &volatility[..] {
                "immutable" => buffer.push_str("IMMUTABLE"),
                "stable" => buffer.push_str("STABLE"),
                "volatile" => buffer.push_str("VOLATILE"),
                unexpected => {
                    return Err(SqlError::Unsupported(format!(
                        "Unsupported volatility: {}",
                        unexpected
                    )))
                }
            }
        } else if name.eq("security") {
            match &**arg {
                Node::Integer { value: 0 } => buffer.push_str("SECURITY INVOKER"),
                Node::Integer { value: 1 } => buffer.push_str("SECURITY DEFINER"),
                unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
            }
        } else if name.eq("leakproof") {
            match &**arg {
                Node::Integer { value: 0 } => buffer.push_str("NOT LEAKPROOF"),
                Node::Integer { value: 1 } => buffer.push_str("LEAKPROOF"),
                unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
            }
        } else if name.eq("cost") {
            buffer.push_str("COST ");
            SqlValue(&**arg).build_with_context(buffer, Context::None)?;
        } else if name.eq("rows") {
            buffer.push_str("ROWS ");
            SqlValue(&**arg).build_with_context(buffer, Context::None)?;
        } else if name.eq("support") {
            buffer.push_str("SUPPORT ");
            let list = node!(**arg, Node::List);
            AnyName(&list.items).build(buffer)?;
        } else if name.eq("set") {
            let stmt = node!(**arg, Node::VariableSetStmt);
            stmt.build(buffer)?;
        } else if name.eq("parallel") {
            buffer.push_str("PARALLEL ");
            buffer.push_str(&quote_identifier(string_value!(**arg)));
        } else {
            return Err(SqlError::Unreachable);
        }

        Ok(())
    }
}

pub(in crate::str) struct VarName<'a>(pub &'a String);
impl SqlBuilder for VarName<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        ColId(self.0).build(buffer)
    }
}

pub(in crate::str) struct VarList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for VarList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, node) in self.0.iter().enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            match node {
                Node::ParamRef(param) => param.build(buffer)?,
                Node::A_Const(a_const) => match &(*a_const.val).0 {
                    Node::Integer { value } => buffer.push_str(&format!("{}", *value)),
                    Node::Float { value: Some(value) } => buffer.push_str(value),
                    Node::String { value: Some(value) } => BooleanOrString(value).build(buffer)?,
                    unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
                },
                unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
            }
        }
        Ok(())
    }
}

pub(in crate::str) struct BooleanOrString<'a>(pub &'a String);
impl SqlBuilder for BooleanOrString<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match &self.0[..] {
            "true" => buffer.push_str("TRUE"),
            "false" => buffer.push_str("FALSE"),
            "on" => buffer.push_str("ON"),
            "off" => buffer.push_str("OFF"),
            value => non_reserved_word_or_sconst(buffer, value)?,
        }
        Ok(())
    }
}

pub(in crate::str) struct OptWith<'a>(pub &'a Vec<Node>);
impl SqlBuilder for OptWith<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if !self.0.is_empty() {
            buffer.push_str("WITH ");
            RelOptions(self.0).build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct OptDropBehavior<'a>(pub &'a DropBehavior);
impl SqlBuilder for OptDropBehavior<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match *self.0 {
            DropBehavior::DROP_RESTRICT => {}
            DropBehavior::DROP_CASCADE => buffer.push_str(" CASCADE"),
        }
        Ok(())
    }
}

pub(in crate::str) struct TransactionModeList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for TransactionModeList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, item) in iter_only!(self.0, Node::DefElem).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }

            let name = must!(item.defname);
            let arg = must!(item.arg);
            let arg = node!(**arg, Node::A_Const);
            match &name[..] {
                "transaction_isolation" => {
                    let value = string_value!((*arg.val).0);
                    buffer.push_str("ISOLATION LEVEL ");
                    match &value[..] {
                        "read uncommitted" => buffer.push_str("READ UNCOMMITTED"),
                        "read committed" => buffer.push_str("READ COMMITTED"),
                        "repeatable read" => buffer.push_str("REPEATABLE READ"),
                        "serializable" => buffer.push_str("SERIALIZABLE"),
                        unsupported => {
                            return Err(SqlError::Unsupported(format!(
                                "Unsupported isolation mode: {}",
                                unsupported
                            )))
                        }
                    }
                }
                "transaction_read_only" => {
                    match &(*arg.val).0 {
                        Node::Integer { value } if *value == 0 => {
                            buffer.push_str("READ WRITE");
                        }
                        Node::Integer { value } if *value == 1 => {
                            buffer.push_str("READ ONLY");
                        }
                        unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
                    };
                }
                "transaction_deferrable" => {
                    match &(*arg.val).0 {
                        Node::Integer { value } if *value == 0 => {
                            buffer.push_str("NOT DEFERRABLE");
                        }
                        Node::Integer { value } if *value == 1 => {
                            buffer.push_str("DEFERRABLE");
                        }
                        unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
                    };
                }
                unsupported => {
                    return Err(SqlError::Unsupported(format!(
                        "Unsupported transaction mode: {}",
                        unsupported
                    )))
                }
            }
        }
        Ok(())
    }
}

pub(in crate::str) struct NumericOnly<'a>(pub &'a Node);
impl SqlBuilder for NumericOnly<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match self.0 {
            Node::Integer { value } => buffer.push_str(&format!("{}", *value)),
            Node::Float { value: Some(value) } => buffer.push_str(value),
            unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
        }
        Ok(())
    }
}

pub(in crate::str) struct SeqOptElem<'a>(pub &'a DefElem);
impl SqlBuilder for SeqOptElem<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.0.defname);
        match &name[..] {
            "as" => {
                let arg = must!(self.0.arg);
                let arg = node!(**arg, Node::TypeName);
                buffer.push_str("AS ");
                arg.build(buffer)?;
            }
            "cache" => {
                let arg = must!(self.0.arg);
                buffer.push_str("CACHE ");
                NumericOnly(&**arg).build(buffer)?;
            }
            "cycle" => {
                let arg = must!(self.0.arg);
                let arg = int_value!(**arg);
                match arg {
                    0 => buffer.push_str("NO CYCLE"),
                    1 => buffer.push_str("CYCLE"),
                    cycle => return Err(SqlError::Unsupported(format!("Cycle: {}", cycle))),
                }
            }
            "increment" => {
                let arg = must!(self.0.arg);
                buffer.push_str("INCREMENT ");
                NumericOnly(&**arg).build(buffer)?;
            }
            "maxvalue" => {
                if let Some(ref arg) = self.0.arg {
                    buffer.push_str("MAXVALUE ");
                    NumericOnly(&**arg).build(buffer)?;
                } else {
                    buffer.push_str("NO MAXVALUE");
                }
            }
            "minvalue" => {
                if let Some(ref arg) = self.0.arg {
                    buffer.push_str("MINVALUE ");
                    NumericOnly(&**arg).build(buffer)?;
                } else {
                    buffer.push_str("NO MINVALUE");
                }
            }
            "owned_by" => {
                let arg = must!(self.0.arg);
                let arg = node!(**arg, Node::List);
                buffer.push_str("OWNED BY ");
                AnyName(&arg.items).build(buffer)?;
            }
            "sequence_name" => {
                let arg = must!(self.0.arg);
                let arg = node!(**arg, Node::List);
                buffer.push_str("SEQUENCE NAME ");
                AnyName(&arg.items).build(buffer)?;
            }
            "start" => {
                let arg = must!(self.0.arg);
                buffer.push_str("START ");
                NumericOnly(&**arg).build(buffer)?;
            }
            "restart" => {
                if let Some(ref arg) = self.0.arg {
                    buffer.push_str("RESTART ");
                    NumericOnly(&**arg).build(buffer)?;
                } else {
                    buffer.push_str("RESTART");
                }
            }
            unsupported => {
                return Err(SqlError::Unsupported(format!(
                    "Option element: {}",
                    unsupported
                )))
            }
        }
        Ok(())
    }
}

pub(in crate::str) struct AlterIdentityColumnOptionList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for AlterIdentityColumnOptionList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, elem) in iter_only!(self.0, Node::DefElem).enumerate() {
            if index > 0 {
                buffer.push(' ');
            }
            let name = must!(elem.defname);
            match &name[..] {
                "restart" => {
                    buffer.push_str("RESTART");
                    if let Some(ref arg) = elem.arg {
                        buffer.push(' ');
                        NumericOnly(&**arg).build(buffer)?;
                    }
                }
                "generated" => {
                    buffer.push_str("SET GENERATED ");
                    let arg = must!(elem.arg);
                    let arg = int_value!(**arg);
                    let arg = char::from(arg as u8);
                    match arg {
                        constants::ATTRIBUTE_IDENTITY_ALWAYS => buffer.push_str("ALWAYS"),
                        constants::ATTRIBUTE_IDENTITY_BY_DEFAULT => buffer.push_str("BY DEFAULT"),
                        _ => unsupported!(arg),
                    }
                }
                _ => {
                    buffer.push_str("SET ");
                    SeqOptElem(elem).build(buffer)?;
                }
            }
        }
        Ok(())
    }
}

pub(in crate::str) struct AlterGenericOptions<'a>(pub &'a Vec<Node>);
impl SqlBuilder for AlterGenericOptions<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("OPTIONS (");

        for (index, elem) in iter_only!(self.0, Node::DefElem).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }

            let name = must!(elem.defname);
            match *elem.defaction {
                DefElemAction::DEFELEM_UNSPEC => {
                    let arg = must!(elem.arg);
                    let arg = string_value!(**arg);
                    buffer.push_str(&quote_identifier(name));
                    buffer.push(' ');
                    StringLiteral(arg).build(buffer)?;
                }
                DefElemAction::DEFELEM_SET => {
                    let arg = must!(elem.arg);
                    let arg = string_value!(**arg);
                    buffer.push_str("SET ");
                    buffer.push_str(&quote_identifier(name));
                    buffer.push(' ');
                    StringLiteral(arg).build(buffer)?;
                }
                DefElemAction::DEFELEM_ADD => {
                    let arg = must!(elem.arg);
                    let arg = string_value!(**arg);
                    buffer.push_str("ADD ");
                    buffer.push_str(&quote_identifier(name));
                    buffer.push(' ');
                    StringLiteral(arg).build(buffer)?;
                }
                DefElemAction::DEFELEM_DROP => {
                    buffer.push_str("DROP ");
                    buffer.push_str(&quote_identifier(name));
                }
            }
        }
        buffer.push(')');
        Ok(())
    }
}

pub(in crate::str) struct SignedIConst<'a>(pub &'a Node);
impl SqlBuilder for SignedIConst<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let val = int_value!(self.0);
        buffer.push_str(&val.to_string());
        Ok(())
    }
}

pub(in crate::str) struct GenericDefElemName<'a>(pub &'a str);
impl SqlBuilder for GenericDefElemName<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str(&self.0.to_uppercase());
        Ok(())
    }
}

pub(in crate::str) struct GroupByList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for GroupByList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, item) in self.0.iter().enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            match item {
                Node::GroupingSet(set) => set.build(buffer)?,
                other => Expr(other).build(buffer)?,
            }
        }
        Ok(())
    }
}

pub(in crate::str) struct SeqOptList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for SeqOptList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, item) in iter_only!(self.0, Node::DefElem).enumerate() {
            if index > 0 {
                buffer.push(' ');
            }
            SeqOptElem(item).build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct TypeList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for TypeList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, item) in iter_only!(self.0, Node::TypeName).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            item.build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct RelationExprList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for RelationExprList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, item) in iter_only!(self.0, Node::RangeVar).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            item.build_with_context(buffer, Context::None)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct QualifiedNameList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for QualifiedNameList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for range in iter_only!(self.0, Node::RangeVar).enumerate() {
            if range.0 > 0 {
                buffer.push_str(", ");
            }
            range.1.build_with_context(buffer, Context::None)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct XmlNamespaceList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for XmlNamespaceList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, item) in iter_only!(self.0, Node::ResTarget).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            if item.name.is_none() {
                buffer.push_str("DEFAULT ");
            }

            let val = must!(item.val);
            Expr(&**val).build(buffer)?;

            if let Some(ref name) = item.name {
                buffer.push_str(" AS ");
                buffer.push_str(&quote_identifier(name));
            }
        }
        Ok(())
    }
}

pub(in crate::str) struct FunctionWithArgTypesList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for FunctionWithArgTypesList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for item in iter_only!(self.0, Node::ObjectWithArgs).enumerate() {
            if item.0 > 0 {
                buffer.push_str(", ");
            }
            FunctionWithArgTypes(item.1).build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct NumericOnlyList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for NumericOnlyList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for item in self.0.iter().enumerate() {
            if item.0 > 0 {
                buffer.push_str(", ");
            }
            NumericOnly(item.1).build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct RoleList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for RoleList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for item in iter_only!(self.0, Node::RoleSpec).enumerate() {
            if item.0 > 0 {
                buffer.push_str(", ");
            }
            item.1.build(buffer)?;
        }
        Ok(())
    }
}

pub(in crate::str) struct AggrArgs<'a>(pub &'a Vec<Node>);
impl SqlBuilder for AggrArgs<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if self.0.is_empty() {
            return Err(SqlError::Unsupported("Empty AggrArgs".into()));
        }

        buffer.push('(');
        if let Node::List(ref args) = self.0[0] {
            if self.0.len() != 2 {
                return Err(SqlError::Unsupported(format!(
                    "Unexpected length for AggrArgs: {}",
                    self.0.len()
                )));
            }
            let order_by_pos = int_value!(self.0[1]) as usize;
            for (index, item) in iter_only!(args.items, Node::FunctionParameter).enumerate() {
                if index == order_by_pos {
                    if index > 0 {
                        buffer.push(' ');
                    }
                    buffer.push_str("ORDER BY ");
                } else if index > 0 {
                    buffer.push_str(", ");
                }
                item.build(buffer)?;
            }

            // Repeat the last direct arg as a ordered arg to handle the
            // simplification done by makeOrderedSetArgs in gram.y
            if order_by_pos == args.items.len() {
                if let Some(item) = args.items.last() {
                    buffer.push_str(" ORDER BY ");
                    node!(item, Node::FunctionParameter).build(buffer)?;
                }
            }
        } else {
            buffer.push('*');
        }
        buffer.push(')');
        Ok(())
    }
}

pub(in crate::str) struct Definition<'a>(pub &'a Vec<Node>);
impl SqlBuilder for Definition<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push('(');
        for (index, item) in iter_only!(self.0, Node::DefElem).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            let name = must!(item.defname);
            buffer.push_str(&quote_identifier(name));
            if let Some(ref arg) = item.arg {
                buffer.push_str(" = ");
                DefArg(&**arg, false).build(buffer)?;
            }
        }
        buffer.push(')');
        Ok(())
    }
}

pub(in crate::str) struct CreatedbOptList<'a>(pub &'a Vec<Node>);
impl SqlBuilder for CreatedbOptList<'_> {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        for (index, elem) in iter_only!(self.0, Node::DefElem).enumerate() {
            if index > 0 {
                buffer.push(' ');
            }
            let name = must!(elem.defname);
            if name.eq("connection_limit") {
                buffer.push_str("CONNECTION LIMIT");
            } else {
                GenericDefElemName(name).build(buffer)?;
            }

            if let Some(ref arg) = elem.arg {
                match &**arg {
                    Node::String {
                        value: Some(ref value),
                    } => {
                        buffer.push(' ');
                        BooleanOrString(value).build(buffer)?;
                    }
                    Node::Integer { value } => buffer.push_str(&format!(" {}", *value)),
                    _ => {}
                }
            } else {
                buffer.push_str(" DEFAULT");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::Node;
    use crate::str::ext::SqlValue;
    use crate::str::{Context, SqlBuilderWithContext};

    #[test]
    fn it_can_convert_bit_string_x_to_sql_value() {
        let node = Node::BitString {
            value: Some("x123".into()),
        };
        let mut buffer = String::new();
        let result = SqlValue(&node).build_with_context(&mut buffer, Context::None);
        assert!(result.is_ok());
        assert_eq!("x'123'", buffer);
    }

    #[test]
    fn it_can_convert_bit_string_b_to_sql_value() {
        let node = Node::BitString {
            value: Some("b123".into()),
        };
        let mut buffer = String::new();
        let result = SqlValue(&node).build_with_context(&mut buffer, Context::None);
        assert!(result.is_ok());
        assert_eq!("b'123'", buffer);
    }
}
