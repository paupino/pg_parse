use crate::ast::*;
use crate::str::ext::*;
use crate::str::helpers::*;
use crate::str::{Context, SqlBuilder, SqlBuilderWithContext, SqlError};

impl SqlBuilder for A_ArrayExpr {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let elements = must!(self.elements);
        buffer.push_str("ARRAY[");
        ExprList(elements).build(buffer)?;
        buffer.push(']');
        Ok(())
    }
}

impl SqlBuilder for A_Const {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        SqlValue(self.val.inner()).build_with_context(buffer, Context::Constant)
    }
}

impl SqlBuilderWithContext for A_Expr {
    fn build_with_context(&self, buffer: &mut String, context: Context) -> Result<(), SqlError> {
        fn need_parenthesis(expr: &Option<Box<Node>>) -> bool {
            match expr {
                Some(node) => matches!(
                    **node,
                    Node::BoolExpr(_) | Node::NullTest(_) | Node::A_Expr(_)
                ),
                None => false,
            }
        }
        let need_left_parens = need_parenthesis(&self.lexpr);
        let need_right_parens = need_parenthesis(&self.rexpr);

        match *self.kind {
            A_Expr_Kind::AEXPR_OP => {
                // Normal Operator
                let need_outer_parens = context == Context::AExpr;
                if need_outer_parens {
                    buffer.push('(');
                }
                if let Some(ref left) = self.lexpr {
                    if need_left_parens {
                        buffer.push('(');
                    }
                    Expr(&*left).build(buffer)?;
                    if need_left_parens {
                        buffer.push(')');
                    }
                    buffer.push(' ');
                }

                // Operator
                let name = must!(self.name);
                QualifiedOperator(name).build(buffer)?;

                if let Some(ref right) = self.rexpr {
                    buffer.push(' ');
                    if need_right_parens {
                        buffer.push('(');
                    }
                    Expr(&*right).build(buffer)?;
                    if need_right_parens {
                        buffer.push(')');
                    }
                }

                if need_outer_parens {
                    buffer.push(')');
                }
            }
            A_Expr_Kind::AEXPR_OP_ANY | A_Expr_Kind::AEXPR_OP_ALL => {
                let left = must!(self.lexpr);
                let right = must!(self.rexpr);
                let name = must!(self.name);

                // x op ANY(y)
                Expr(&**left).build(buffer)?;
                buffer.push(' ');
                SubqueryOperator(name).build(buffer)?;
                if (*self.kind).eq(&A_Expr_Kind::AEXPR_OP_ALL) {
                    buffer.push_str(" ALL(");
                } else {
                    buffer.push_str(" ANY(");
                }
                Expr(&**right).build(buffer)?;
                buffer.push(')');
            }
            A_Expr_Kind::AEXPR_DISTINCT => {
                let left = must!(self.lexpr);
                let right = must!(self.rexpr);

                if need_left_parens {
                    buffer.push('(');
                }
                Expr(&**left).build(buffer)?;
                if need_left_parens {
                    buffer.push(')');
                }
                buffer.push_str(" IS DISTINCT FROM ");
                if need_right_parens {
                    buffer.push('(');
                }
                Expr(&**right).build(buffer)?;
                if need_right_parens {
                    buffer.push(')');
                }
            }
            A_Expr_Kind::AEXPR_NOT_DISTINCT => {
                let left = must!(self.lexpr);
                let right = must!(self.rexpr);
                Expr(&**left).build(buffer)?;
                buffer.push_str(" IS NOT DISTINCT FROM ");
                Expr(&**right).build(buffer)?;
            }
            A_Expr_Kind::AEXPR_NULLIF => {
                let left = must!(self.lexpr);
                let right = must!(self.rexpr);
                let name = must!(self.name);
                let name = node_vec_to_string_vec(name);
                if name.len() != 1 {
                    return Err(SqlError::Unsupported("name.len() != 1".into()));
                }
                let name = name[0];
                if name.ne("=") {
                    return Err(SqlError::Unsupported("name != '='".into()));
                }

                // Build the expression
                buffer.push_str("NULLIF(");
                Expr(&**left).build(buffer)?;
                buffer.push_str(", ");
                Expr(&**right).build(buffer)?;
                buffer.push(')');
            }
            A_Expr_Kind::AEXPR_OF => {
                let left = must!(self.lexpr);
                let right = must!(self.rexpr);
                let right = node!(**right, Node::List);
                let name = must!(self.name);
                if name.is_empty() {
                    return Err(SqlError::Unsupported("Empty name for AEXPR_OF".into()));
                }
                let name = string_value!(name[0]);

                Expr(&**left).build(buffer)?;
                if name.eq("=") {
                    buffer.push_str(" IS OF ");
                } else if name.eq("<>") {
                    buffer.push_str(" IS NOT OF ");
                } else {
                    return Err(SqlError::Unsupported(format!(
                        "Unexpected operator for AEXPR_OF: {}",
                        name
                    )));
                }
                buffer.push('(');
                TypeList(&right.items).build(buffer)?;
                buffer.push(')');
            }
            A_Expr_Kind::AEXPR_IN => {
                let left = must!(self.lexpr);
                let right = must!(self.rexpr);
                let name = must!(self.name);
                let name = node_vec_to_string_vec(name);
                if name.len() != 1 {
                    return Err(SqlError::Unsupported("name.len() != 1".into()));
                }
                let name = name[0];

                // Start with the left
                Expr(&**left).build(buffer)?;
                buffer.push(' ');
                match &name[..] {
                    "=" => buffer.push_str("IN "),
                    "<>" => buffer.push_str("NOT IN "),
                    unsupported => {
                        return Err(SqlError::Unsupported(format!(
                            "Unsupported operator: {}",
                            unsupported
                        )))
                    }
                }
                buffer.push('(');
                match &**right {
                    Node::SubLink(link) => link.build(buffer)?,
                    Node::List(list) => ExprList(&list.items).build(buffer)?,
                    other => return Err(SqlError::UnexpectedNodeType(other.name())),
                }
                buffer.push(')');
            }
            A_Expr_Kind::AEXPR_LIKE | A_Expr_Kind::AEXPR_ILIKE => {
                let ilike = (*self.kind).eq(&A_Expr_Kind::AEXPR_ILIKE);
                let left = must!(self.lexpr);
                let right = must!(self.rexpr);

                // Start with the left hand side
                Expr(&**left).build(buffer)?;
                buffer.push(' ');

                // Get the operator name
                let name = must!(self.name);
                if name.len() != 1 {
                    return Err(SqlError::Unsupported("name.len() != 1".into()));
                }
                let name = string_value!(name[0]);
                match &name[..] {
                    "~~" if !ilike => buffer.push_str("LIKE "),
                    "~~*" if ilike => buffer.push_str("ILIKE "),
                    "!~~" if !ilike => buffer.push_str("NOT LIKE "),
                    "!~~*" if ilike => buffer.push_str("NOT ILIKE "),
                    op => {
                        return Err(SqlError::Unsupported(format!(
                            "Unsupported operator: {}",
                            op
                        )))
                    }
                }

                // Finish up with the right hand side
                Expr(&**right).build(buffer)?;
            }
            A_Expr_Kind::AEXPR_SIMILAR => {
                let left = must!(self.lexpr);
                let right = must!(self.rexpr);
                let right = node!(**right, Node::FuncCall);
                let name = must!(self.name);
                if name.is_empty() {
                    return Err(SqlError::Unsupported("Empty name for AEXPR_OF".into()));
                }
                let name = string_value!(name[0]);

                Expr(&**left).build(buffer)?;
                if name.eq("~") {
                    buffer.push_str(" SIMILAR TO ");
                } else if name.eq("!~") {
                    buffer.push_str(" NOT SIMILAR TO ");
                } else {
                    return Err(SqlError::Unsupported(format!(
                        "Unexpected operator for AEXPR_SIMILAR: {}",
                        name
                    )));
                }

                // Function call assertions
                let func_name = must!(right.funcname);
                let func_name = node_vec_to_string_vec(func_name);
                if func_name.len() != 2 {
                    return Err(SqlError::Unsupported(format!(
                        "Invalid func_name for AEXPR_SIMILAR: {:?}",
                        func_name
                    )));
                }
                if func_name[0].ne("pg_catalog") || func_name[1].ne("similar_to_escape") {
                    return Err(SqlError::Unsupported(format!(
                        "Invalid func_name for AEXPR_SIMILAR: {:?}",
                        func_name
                    )));
                }
                let args = must!(right.args);
                if args.is_empty() || args.len() > 2 {
                    return Err(SqlError::Unsupported(format!(
                        "Invalid func args for AEXPR_SIMILAR: {}",
                        args.len()
                    )));
                }
                let mut args = args.iter();

                Expr(args.next().unwrap()).build(buffer)?;
                if let Some(arg) = args.next() {
                    buffer.push_str(" ESCAPE ");
                    Expr(arg).build(buffer)?;
                }
            }
            A_Expr_Kind::AEXPR_BETWEEN
            | A_Expr_Kind::AEXPR_NOT_BETWEEN
            | A_Expr_Kind::AEXPR_BETWEEN_SYM
            | A_Expr_Kind::AEXPR_NOT_BETWEEN_SYM => {
                let left = must!(self.lexpr);

                // RHS must be a list
                let right = must!(self.rexpr);
                let right = node!(**right, Node::List);

                let name = must!(self.name);
                let name = node_vec_to_string_vec(name);
                if name.len() != 1 {
                    return Err(SqlError::Unsupported("name.len() != 1".into()));
                }
                let name = name[0];

                // Build the expression
                Expr(&**left).build(buffer)?;
                buffer.push_str(&format!(" {} ", name));

                let mut iter = right.items.iter().peekable();
                while let Some(expr) = iter.next() {
                    Expr(expr).build(buffer)?;
                    if iter.peek().is_some() {
                        buffer.push_str(" AND ");
                    }
                }
            }
            A_Expr_Kind::AEXPR_PAREN => {
                // Dummy node for parenthesis
                return Err(SqlError::Unsupported("AEXPR_PAREN".into()));
            }
        }
        Ok(())
    }
}

impl SqlBuilder for A_Indices {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push('[');
        if let Some(ref index) = self.lidx {
            Expr(&**index).build(buffer)?;
        }
        if self.is_slice {
            buffer.push(':');
        }
        if let Some(ref index) = self.uidx {
            Expr(&**index).build(buffer)?;
        }
        buffer.push(']');
        Ok(())
    }
}

impl SqlBuilder for A_Indirection {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let arg = must!(self.arg);
        let empty = Vec::new();
        let indirection = self.indirection.as_ref().unwrap_or(&empty);

        let indirection_indices = if !indirection.is_empty() {
            matches!(indirection[0], Node::A_Indices(_))
        } else {
            false
        };
        let parenthesis = matches!(
            **arg,
            Node::A_Indirection(_)
                | Node::FuncCall(_)
                | Node::A_Expr(_)
                | Node::TypeCast(_)
                | Node::RowExpr(_)
        ) || (matches!(**arg, Node::ColumnRef(_)) && !indirection_indices);

        // Process expression
        if parenthesis {
            buffer.push('(');
        }
        Expr(&**arg).build(buffer)?;
        if parenthesis {
            buffer.push(')');
        }

        Indirection(indirection, 0).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for A_Star {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push('*');
        Ok(())
    }
}

impl SqlBuilder for AccessPriv {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if let Some(ref name) = self.priv_name {
            match &name[..] {
                "select" | "references" | "create" => buffer.push_str(name),
                other => buffer.push_str(&quote_identifier(other)),
            }
        } else {
            buffer.push_str("ALL");
        }

        if let Some(ref cols) = self.cols {
            if !cols.is_empty() {
                buffer.push_str(" (");
                ColumnList(cols).build(buffer)?;
                buffer.push(')');
            }
        }
        Ok(())
    }
}

impl SqlBuilder for AlterSystemStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let stmt = must!(self.setstmt);
        buffer.push_str("ALTER SYSTEM ");
        (**stmt).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilderWithContext for AlterTableCmd {
    fn build_with_context(
        &self,
        buffer: &mut String,
        context: Context,
    ) -> core::result::Result<(), SqlError> {
        let mut options = None;
        let mut trailing_missing_ok = false;

        match *self.subtype {
            AlterTableType::AT_AddColumn => {
                if context == Context::AlterType {
                    buffer.push_str("ADD ATTRIBUTE");
                } else {
                    buffer.push_str("ADD COLUMN");
                }
            }
            AlterTableType::AT_AddColumnRecurse => unsupported!(self.subtype),
            AlterTableType::AT_AddColumnToView => unsupported!(self.subtype),
            AlterTableType::AT_ColumnDefault => {
                buffer.push_str("ALTER COLUMN");
                if self.def.is_some() {
                    options = Some("SET DEFAULT");
                } else {
                    options = Some("DROP DEFAULT");
                }
            }
            AlterTableType::AT_CookedColumnDefault => unsupported!(self.subtype),
            AlterTableType::AT_DropNotNull => {
                buffer.push_str("ALTER COLUMN");
                options = Some("DROP NOT NULL");
            }
            AlterTableType::AT_SetNotNull => {
                buffer.push_str("ALTER COLUMN");
                options = Some("SET NOT NULL");
            }
            AlterTableType::AT_DropExpression => {
                buffer.push_str("ALTER COLUMN");
                options = Some("DROP EXPRESSION");
                trailing_missing_ok = true;
            }
            AlterTableType::AT_CheckNotNull => unsupported!(self.subtype),
            AlterTableType::AT_SetStatistics => {
                buffer.push_str("ALTER COLUMN");
                options = Some("SET STATISTICS");
            }
            AlterTableType::AT_SetOptions => {
                buffer.push_str("ALTER COLUMN");
                options = Some("SET");
            }
            AlterTableType::AT_ResetOptions => {
                buffer.push_str("ALTER COLUMN");
                options = Some("RESET");
            }
            AlterTableType::AT_SetStorage => {
                buffer.push_str("ALTER COLUMN");
                options = Some("SET STORAGE");
            }
            AlterTableType::AT_DropColumn => {
                if context == Context::AlterType {
                    buffer.push_str("DROP ATTRIBUTE");
                } else {
                    buffer.push_str("DROP COLUMN");
                }
            }
            AlterTableType::AT_DropColumnRecurse => unsupported!(self.subtype),
            AlterTableType::AT_AddIndex => buffer.push_str("ADD INDEX"),
            AlterTableType::AT_ReAddIndex => unsupported!(self.subtype),
            AlterTableType::AT_AddConstraint => buffer.push_str("ADD"),
            AlterTableType::AT_AddConstraintRecurse => unsupported!(self.subtype),
            AlterTableType::AT_ReAddConstraint => unsupported!(self.subtype),
            AlterTableType::AT_ReAddDomainConstraint => unsupported!(self.subtype),
            AlterTableType::AT_AlterConstraint => buffer.push_str("ALTER"),
            AlterTableType::AT_ValidateConstraint => buffer.push_str("VALIDATE CONSTRAINT"),
            AlterTableType::AT_ValidateConstraintRecurse => unsupported!(self.subtype),
            AlterTableType::AT_AddIndexConstraint => unsupported!(self.subtype),
            AlterTableType::AT_DropConstraint => buffer.push_str("DROP CONSTRAINT"),
            AlterTableType::AT_DropConstraintRecurse => unsupported!(self.subtype),
            AlterTableType::AT_ReAddComment => unsupported!(self.subtype),
            AlterTableType::AT_AlterColumnType => {
                if context == Context::AlterType {
                    buffer.push_str("ALTER ATTRIBUTE");
                } else {
                    buffer.push_str("ALTER COLUMN");
                }
                options = Some("TYPE");
            }
            AlterTableType::AT_AlterColumnGenericOptions => buffer.push_str("ALTER COLUMN"),
            AlterTableType::AT_ChangeOwner => {
                buffer.push_str("OWNER TO ");
                must!(self.newowner).build(buffer)?;
            }
            AlterTableType::AT_ClusterOn => buffer.push_str("CLUSTER ON"),
            AlterTableType::AT_DropCluster => buffer.push_str("SET WITHOUT CLUSTER"),
            AlterTableType::AT_SetLogged => buffer.push_str("SET LOGGED"),
            AlterTableType::AT_SetUnLogged => buffer.push_str("SET UNLOGGED"),
            AlterTableType::AT_DropOids => buffer.push_str("SET WITHOUT OIDS"),
            AlterTableType::AT_SetTableSpace => buffer.push_str("SET TABLESPACE"),
            AlterTableType::AT_SetRelOptions => buffer.push_str("SET"),
            AlterTableType::AT_ResetRelOptions => buffer.push_str("RESET"),
            AlterTableType::AT_ReplaceRelOptions => unsupported!(self.subtype),
            AlterTableType::AT_EnableTrig => buffer.push_str("ENABLE TRIGGER"),
            AlterTableType::AT_EnableAlwaysTrig => buffer.push_str("ENABLE ALWAYS TRIGGER"),
            AlterTableType::AT_EnableReplicaTrig => buffer.push_str("ENABLE REPLICA TRIGGER"),
            AlterTableType::AT_DisableTrig => buffer.push_str("DISABLE TRIGGER"),
            AlterTableType::AT_EnableTrigAll => buffer.push_str("ENABLE TRIGGER"),
            AlterTableType::AT_DisableTrigAll => buffer.push_str("DISABLE TRIGGER ALL"),
            AlterTableType::AT_EnableTrigUser => buffer.push_str("ENABLE TRIGGER USER"),
            AlterTableType::AT_DisableTrigUser => buffer.push_str("DISABLE TRIGGER USER"),
            AlterTableType::AT_EnableRule => buffer.push_str("ENABLE RULE"),
            AlterTableType::AT_EnableAlwaysRule => buffer.push_str("ENABLE ALWAYS RULE"),
            AlterTableType::AT_EnableReplicaRule => buffer.push_str("ENABLE REPLICA RULE"),
            AlterTableType::AT_DisableRule => buffer.push_str("DISABLE RULE"),
            AlterTableType::AT_AddInherit => buffer.push_str("INHERIT"),
            AlterTableType::AT_DropInherit => buffer.push_str("NO INHERIT"),
            AlterTableType::AT_AddOf => buffer.push_str("OF"),
            AlterTableType::AT_DropOf => buffer.push_str("NOT OF"),
            AlterTableType::AT_ReplicaIdentity => buffer.push_str("REPLICA IDENTITY"),
            AlterTableType::AT_EnableRowSecurity => buffer.push_str("ENABLE ROW LEVEL SECURITY"),
            AlterTableType::AT_DisableRowSecurity => buffer.push_str("DISABLE ROW LEVEL SECURITY"),
            AlterTableType::AT_ForceRowSecurity => buffer.push_str("FORCE ROW LEVEL SECURITY"),
            AlterTableType::AT_NoForceRowSecurity => buffer.push_str("NO FORCE ROW LEVEL SECURITY"),
            AlterTableType::AT_GenericOptions => {} // Handled in def field handling
            AlterTableType::AT_AttachPartition => buffer.push_str("ATTACH PARTITION"),
            AlterTableType::AT_DetachPartition => buffer.push_str("DETACH PARTITION"),
            AlterTableType::AT_AddIdentity => {
                buffer.push_str("ALTER");
                options = Some("ADD");
            }
            AlterTableType::AT_SetIdentity => buffer.push_str("ALTER"),
            AlterTableType::AT_DropIdentity => {
                buffer.push_str("ALTER COLUMN");
                options = Some("DROP IDENTITY");
                trailing_missing_ok = true;
            }
        }

        if self.missing_ok && !trailing_missing_ok {
            if *self.subtype == AlterTableType::AT_AddColumn {
                buffer.push_str(" IF NOT EXISTS");
            } else {
                buffer.push_str(" IF EXISTS");
            }
        }

        if let Some(ref name) = self.name {
            buffer.push(' ');
            buffer.push_str(&quote_identifier(name));
        }

        if self.num > 0 {
            buffer.push_str(&format!(" {}", self.num));
        }

        if let Some(options) = options {
            buffer.push(' ');
            buffer.push_str(options);
        }

        if self.missing_ok && trailing_missing_ok {
            buffer.push_str(" IF EXISTS");
        }

        match *self.subtype {
            AlterTableType::AT_AttachPartition | AlterTableType::AT_DetachPartition => {
                let def = must!(self.def);
                let cmd = node!(**def, Node::PartitionCmd);
                buffer.push(' ');
                cmd.build(buffer)?;
            }
            AlterTableType::AT_AddColumn | AlterTableType::AT_AlterColumnType => {
                let def = must!(self.def);
                let column = node!(**def, Node::ColumnDef);
                buffer.push(' ');
                column.build(buffer)?;
            }
            AlterTableType::AT_ColumnDefault => {
                if let Some(ref def) = self.def {
                    buffer.push(' ');
                    Expr(&**def).build(buffer)?;
                }
            }
            AlterTableType::AT_SetStatistics => {
                buffer.push(' ');
                let def = must!(self.def);
                SignedIConst(&**def).build(buffer)?;
            }
            AlterTableType::AT_SetOptions
            | AlterTableType::AT_ResetOptions
            | AlterTableType::AT_SetRelOptions
            | AlterTableType::AT_ResetRelOptions => {
                buffer.push(' ');
                let def = must!(self.def);
                let def = node!(**def, Node::List);
                RelOptions(&def.items).build(buffer)?;
            }
            AlterTableType::AT_SetStorage => {
                let def = must!(self.def);
                let col_id = string_value!(**def);
                buffer.push(' ');
                ColId(col_id).build(buffer)?;
            }
            AlterTableType::AT_AddIdentity
            | AlterTableType::AT_AddConstraint
            | AlterTableType::AT_AlterConstraint => {
                let def = must!(self.def);
                let constraint = node!(**def, Node::Constraint);
                buffer.push(' ');
                constraint.build(buffer)?;
            }
            AlterTableType::AT_SetIdentity => {
                let def = must!(self.def);
                let list = node!(**def, Node::List);
                buffer.push(' ');
                AlterIdentityColumnOptionList(&list.items).build(buffer)?;
            }
            AlterTableType::AT_AlterColumnGenericOptions | AlterTableType::AT_GenericOptions => {
                let def = must!(self.def);
                let list = node!(**def, Node::List);
                buffer.push(' ');
                AlterGenericOptions(&list.items).build(buffer)?;
            }
            AlterTableType::AT_AddInherit | AlterTableType::AT_DropInherit => {
                let def = must!(self.def);
                let range = node!(**def, Node::RangeVar);
                buffer.push(' ');
                range.build_with_context(buffer, Context::None)?;
            }
            AlterTableType::AT_AddOf => {
                let def = must!(self.def);
                let type_name = node!(**def, Node::TypeName);
                buffer.push(' ');
                type_name.build(buffer)?;
            }
            AlterTableType::AT_ReplicaIdentity => {
                let def = must!(self.def);
                let stmt = node!(**def, Node::ReplicaIdentityStmt);
                buffer.push(' ');
                stmt.build(buffer)?;
            }
            _ => {
                if self.def.is_some() {
                    return Err(SqlError::Unsupported("Unsupported def".into()));
                }
            }
        }

        // Cascade if need be. This adds a space if necessary.
        OptDropBehavior(&*self.behavior).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for AlterDatabaseSetStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let dbname = must!(self.dbname);
        let stmt = must!(self.setstmt);
        buffer.push_str("ALTER DATABASE ");
        ColId(dbname).build(buffer)?;
        buffer.push(' ');
        stmt.build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for AlterDatabaseStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let dbname = must!(self.dbname);
        let options = must!(self.options);
        buffer.push_str("ALTER DATABASE ");
        ColId(dbname).build(buffer)?;
        buffer.push(' ');
        CreatedbOptList(options).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for AlterExtensionContentsStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.extname);
        buffer.push_str("ALTER EXTENSION ");
        ColId(name).build(buffer)?;

        if self.action == 1 {
            buffer.push_str(" ADD ");
        } else if self.action == -1 {
            buffer.push_str(" DROP ");
        } else {
            return Err(SqlError::Unsupported(format!(
                "Unsupported action: {}",
                self.action
            )));
        }

        match *self.objtype {
            ObjectType::OBJECT_ACCESS_METHOD => buffer.push_str("ACCESS METHOD "),
            ObjectType::OBJECT_AGGREGATE => buffer.push_str("AGGREGATE "),
            ObjectType::OBJECT_CAST => buffer.push_str("CAST "),
            ObjectType::OBJECT_COLLATION => buffer.push_str("COLLATION "),
            ObjectType::OBJECT_CONVERSION => buffer.push_str("CONVERSION "),
            ObjectType::OBJECT_DOMAIN => buffer.push_str("DOMAIN "),
            ObjectType::OBJECT_FUNCTION => buffer.push_str("FUNCTION "),
            ObjectType::OBJECT_LANGUAGE => buffer.push_str("LANGUAGE "),
            ObjectType::OBJECT_OPERATOR => buffer.push_str("OPERATOR "),
            ObjectType::OBJECT_OPCLASS => buffer.push_str("OPERATOR CLASS "),
            ObjectType::OBJECT_OPFAMILY => buffer.push_str("OPERATOR FAMILY "),
            ObjectType::OBJECT_PROCEDURE => buffer.push_str("PROCEDURE "),
            ObjectType::OBJECT_ROUTINE => buffer.push_str("ROUTINE "),
            ObjectType::OBJECT_SCHEMA => buffer.push_str("SCHEMA "),
            ObjectType::OBJECT_EVENT_TRIGGER => buffer.push_str("EVENT TRIGGER "),
            ObjectType::OBJECT_TABLE => buffer.push_str("TABLE "),
            ObjectType::OBJECT_TSPARSER => buffer.push_str("TEXT SEARCH PARSER "),
            ObjectType::OBJECT_TSDICTIONARY => buffer.push_str("TEXT SEARCH DICTIONARY "),
            ObjectType::OBJECT_TSTEMPLATE => buffer.push_str("TEXT SEARCH TEMPLATE "),
            ObjectType::OBJECT_TSCONFIGURATION => buffer.push_str("TEXT SEARCH CONFIGURATION "),
            ObjectType::OBJECT_SEQUENCE => buffer.push_str("SEQUENCE "),
            ObjectType::OBJECT_VIEW => buffer.push_str("VIEW "),
            ObjectType::OBJECT_MATVIEW => buffer.push_str("MATERIALIZED VIEW "),
            ObjectType::OBJECT_FOREIGN_TABLE => buffer.push_str("FOREIGN TABLE "),
            ObjectType::OBJECT_FDW => buffer.push_str("FOREIGN DATA WRAPPER "),
            ObjectType::OBJECT_FOREIGN_SERVER => buffer.push_str("SERVER "),
            ObjectType::OBJECT_TRANSFORM => buffer.push_str("TRANSFORM "),
            ObjectType::OBJECT_TYPE => buffer.push_str("TYPE "),
            unexpected => unsupported!(unexpected),
        }

        let object = must!(self.object);
        match *self.objtype {
            ObjectType::OBJECT_COLLATION
            | ObjectType::OBJECT_CONVERSION
            | ObjectType::OBJECT_TABLE
            | ObjectType::OBJECT_TSPARSER
            | ObjectType::OBJECT_TSDICTIONARY
            | ObjectType::OBJECT_TSTEMPLATE
            | ObjectType::OBJECT_TSCONFIGURATION
            | ObjectType::OBJECT_SEQUENCE
            | ObjectType::OBJECT_VIEW
            | ObjectType::OBJECT_MATVIEW
            | ObjectType::OBJECT_FOREIGN_TABLE => {
                let list = node!(**object, Node::List);
                AnyName(&list.items).build(buffer)?;
            }
            ObjectType::OBJECT_ACCESS_METHOD
            | ObjectType::OBJECT_LANGUAGE
            | ObjectType::OBJECT_SCHEMA
            | ObjectType::OBJECT_EVENT_TRIGGER
            | ObjectType::OBJECT_FDW
            | ObjectType::OBJECT_FOREIGN_SERVER => {
                let value = string_value!(**object);
                ColId(value).build(buffer)?;
            }
            ObjectType::OBJECT_AGGREGATE => {
                let arg = node!(**object, Node::ObjectWithArgs);
                AggregateWithArgTypes(arg).build(buffer)?;
            }
            ObjectType::OBJECT_FUNCTION
            | ObjectType::OBJECT_PROCEDURE
            | ObjectType::OBJECT_ROUTINE => {
                let arg = node!(**object, Node::ObjectWithArgs);
                FunctionWithArgTypes(arg).build(buffer)?;
            }
            ObjectType::OBJECT_OPERATOR => {
                let arg = node!(**object, Node::ObjectWithArgs);
                OperatorWithArgTypes(arg).build(buffer)?;
            }
            ObjectType::OBJECT_CAST => {
                let list = node!(**object, Node::List);
                if list.items.len() != 2 {
                    return Err(SqlError::Unsupported(format!(
                        "Expected object len 2 for {:?}: {}",
                        self.objtype,
                        list.items.len()
                    )));
                }
                buffer.push('(');
                node!(list.items[0], Node::TypeName).build(buffer)?;
                buffer.push_str(" AS ");
                node!(list.items[1], Node::TypeName).build(buffer)?;
                buffer.push(')');
            }
            ObjectType::OBJECT_DOMAIN | ObjectType::OBJECT_TYPE => {
                let type_name = node!(**object, Node::TypeName);
                type_name.build(buffer)?;
            }
            ObjectType::OBJECT_OPFAMILY | ObjectType::OBJECT_OPCLASS => {
                let list = node!(**object, Node::List);
                if list.items.len() != 2 {
                    return Err(SqlError::Unsupported(format!(
                        "Expected object len 2 for {:?}: {}",
                        self.objtype,
                        list.items.len()
                    )));
                }
                AnyName(&list.items[1..]).build(buffer)?;
                buffer.push_str(" USING ");
                let val = string_value!(list.items[0]);
                ColId(val).build(buffer)?;
            }
            ObjectType::OBJECT_TRANSFORM => {
                let list = node!(**object, Node::List);
                if list.items.len() != 2 {
                    return Err(SqlError::Unsupported(format!(
                        "Expected object len 2 for {:?}: {}",
                        self.objtype,
                        list.items.len()
                    )));
                }
                buffer.push_str("FOR ");
                node!(list.items[0], Node::TypeName).build(buffer)?;
                buffer.push_str(" LANGUAGE ");
                ColId(string_value!(list.items[1])).build(buffer)?;
            }
            unexpected => unsupported!(unexpected),
        }

        Ok(())
    }
}

impl SqlBuilder for AlterExtensionStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.extname);
        buffer.push_str("ALTER EXTENSION ");
        ColId(name).build(buffer)?;
        buffer.push_str(" UPDATE");
        if let Some(ref options) = self.options {
            for opt in iter_only!(options, Node::DefElem) {
                let opt_name = must!(opt.defname);
                if opt_name.eq("new_version") {
                    let arg = must!(opt.arg);
                    buffer.push_str(" TO ");
                    NonReservedWordOrSconst(&**arg).build(buffer)?;
                }
            }
        }
        Ok(())
    }
}

impl SqlBuilder for AlterObjectDependsStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let extname = must!(self.extname);

        buffer.push_str("ALTER ");
        match *self.object_type {
            ObjectType::OBJECT_FUNCTION => {
                let args = must!(self.object);
                let args = node!(**args, Node::ObjectWithArgs);
                buffer.push_str("FUNCTION ");
                FunctionWithArgTypes(args).build(buffer)?;
            }
            ObjectType::OBJECT_PROCEDURE => {
                let args = must!(self.object);
                let args = node!(**args, Node::ObjectWithArgs);
                buffer.push_str("PROCEDURE ");
                FunctionWithArgTypes(args).build(buffer)?;
            }
            ObjectType::OBJECT_ROUTINE => {
                let args = must!(self.object);
                let args = node!(**args, Node::ObjectWithArgs);
                buffer.push_str("ROUTINE ");
                FunctionWithArgTypes(args).build(buffer)?;
            }
            ObjectType::OBJECT_TRIGGER => {
                let list = must!(self.object);
                let list = node!(**list, Node::List);
                if list.items.is_empty() {
                    return Err(SqlError::Unsupported("Empty list".into()));
                }
                let relation = must!(self.relation);

                buffer.push_str("TRIGGER ");
                ColId(string_value!(list.items[0])).build(buffer)?;
                buffer.push_str(" ON ");
                (**relation).build_with_context(buffer, Context::None)?;
            }
            ObjectType::OBJECT_MATVIEW => {
                let relation = must!(self.relation);

                buffer.push_str("MATERIALIZED VIEW ");
                (**relation).build_with_context(buffer, Context::None)?;
            }
            ObjectType::OBJECT_INDEX => {
                let relation = must!(self.relation);

                buffer.push_str("INDEX ");
                (**relation).build_with_context(buffer, Context::None)?;
            }
            unexpected => unsupported!(unexpected),
        }

        if self.remove {
            buffer.push_str(" NO");
        }
        buffer.push_str(" DEPENDS ON EXTENSION ");
        ColId(string_value!(extname.0)).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for AlterObjectSchemaStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let relation = must!(self.relation);
        let new_schema = must!(self.newschema);

        buffer.push_str("ALTER");
        match *self.object_type {
            ObjectType::OBJECT_ACCESS_METHOD => buffer.push_str(" ACCESS METHOD"),
            ObjectType::OBJECT_AGGREGATE => buffer.push_str(" AGGREGATE"),
            ObjectType::OBJECT_CAST => buffer.push_str(" CAST"),
            ObjectType::OBJECT_COLLATION => buffer.push_str(" COLLATION"),
            ObjectType::OBJECT_CONVERSION => buffer.push_str(" CONVERSION"),
            ObjectType::OBJECT_DOMAIN => buffer.push_str(" DOMAIN"),
            ObjectType::OBJECT_EVENT_TRIGGER => buffer.push_str(" EVENT TRIGGER"),
            ObjectType::OBJECT_EXTENSION => buffer.push_str(" EXTENSION"),
            ObjectType::OBJECT_FDW => buffer.push_str(" FOREIGN DATA WRAPPER"),
            ObjectType::OBJECT_FOREIGN_SERVER => buffer.push_str(" SERVER"),
            ObjectType::OBJECT_FOREIGN_TABLE => buffer.push_str(" FOREIGN TABLE"),
            ObjectType::OBJECT_FUNCTION => buffer.push_str(" FUNCTION"),
            ObjectType::OBJECT_INDEX => buffer.push_str(" INDEX"),
            ObjectType::OBJECT_LANGUAGE => buffer.push_str(" LANGUAGE"),
            ObjectType::OBJECT_MATVIEW => buffer.push_str(" MATERIALIZED VIEW"),
            ObjectType::OBJECT_OPCLASS => buffer.push_str(" OPERATOR CLASS"),
            ObjectType::OBJECT_OPERATOR => buffer.push_str(" OPERATOR"),
            ObjectType::OBJECT_OPFAMILY => buffer.push_str(" OPERATOR FAMILY"),
            ObjectType::OBJECT_POLICY => buffer.push_str(" POLICY"),
            ObjectType::OBJECT_PROCEDURE => buffer.push_str(" PROCEDURE"),
            ObjectType::OBJECT_PUBLICATION => buffer.push_str(" PUBLICATION"),
            ObjectType::OBJECT_ROUTINE => buffer.push_str(" ROUTINE"),
            ObjectType::OBJECT_RULE => buffer.push_str(" RULE"),
            ObjectType::OBJECT_SCHEMA => buffer.push_str(" SCHEMA"),
            ObjectType::OBJECT_SEQUENCE => buffer.push_str(" SEQUENCE"),
            ObjectType::OBJECT_STATISTIC_EXT => buffer.push_str(" STATISTICS"),
            ObjectType::OBJECT_TABLE => buffer.push_str(" TABLE"),
            ObjectType::OBJECT_TRANSFORM => buffer.push_str(" TRANSFORM"),
            ObjectType::OBJECT_TRIGGER => buffer.push_str(" TRIGGER"),
            ObjectType::OBJECT_TSCONFIGURATION => buffer.push_str(" TEXT SEARCH CONFIGURATION"),
            ObjectType::OBJECT_TSDICTIONARY => buffer.push_str(" TEXT SEARCH DICTIONARY"),
            ObjectType::OBJECT_TSPARSER => buffer.push_str(" TEXT SEARCH PARSER"),
            ObjectType::OBJECT_TSTEMPLATE => buffer.push_str(" TEXT SEARCH TEMPLATE"),
            ObjectType::OBJECT_TYPE => buffer.push_str(" TYPE"),
            ObjectType::OBJECT_VIEW => buffer.push_str(" VIEW"),
            unsupported => return Err(SqlError::Unsupported(format!("{:?}", unsupported))),
        }
        if self.missing_ok {
            buffer.push_str(" IF EXISTS");
        }
        buffer.push(' ');
        (**relation).build_with_context(buffer, Context::None)?;
        buffer.push_str(" SET SCHEMA ");
        buffer.push_str(new_schema);
        Ok(())
    }
}

impl SqlBuilder for AlterTableSpaceOptionsStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.tablespacename);
        let options = must!(self.options);

        buffer.push_str("ALTER TABLESPACE ");
        ColId(name).build(buffer)?;

        if self.is_reset {
            buffer.push_str(" RESET ");
        } else {
            buffer.push_str(" SET ");
        }

        RelOptions(options).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for AlterTableStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let relation = must!(self.relation);
        let commands = must!(self.cmds);

        let mut context = Context::None;

        buffer.push_str("ALTER");
        match *self.relkind {
            ObjectType::OBJECT_TABLE => buffer.push_str(" TABLE"),
            ObjectType::OBJECT_FOREIGN_TABLE => buffer.push_str(" FOREIGN TABLE"),
            ObjectType::OBJECT_INDEX => buffer.push_str(" INDEX"),
            ObjectType::OBJECT_SEQUENCE => buffer.push_str(" SEQUENCE"),
            ObjectType::OBJECT_VIEW => buffer.push_str(" VIEW"),
            ObjectType::OBJECT_MATVIEW => buffer.push_str(" MATERIALIZED VIEW"),
            ObjectType::OBJECT_TYPE => {
                buffer.push_str(" TYPE");
                context = Context::AlterType;
            }
            unexpected => return Err(SqlError::UnexpectedObjectType(unexpected)),
        }

        if self.missing_ok {
            buffer.push_str(" IF EXISTS");
        }

        buffer.push(' ');
        (**relation).build_with_context(buffer, context)?;

        for (index, cmd) in iter_only!(commands, Node::AlterTableCmd).enumerate() {
            if index == 0 {
                buffer.push(' ');
            } else {
                buffer.push_str(", ");
            }
            cmd.build_with_context(buffer, context)?;
        }
        Ok(())
    }
}

impl SqlBuilder for CollateClause {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.collname);

        if let Some(ref arg) = self.arg {
            let paren = matches!(**arg, Node::A_Expr(_));
            if paren {
                buffer.push('(');
            }
            Expr(&**arg).build(buffer)?;
            if paren {
                buffer.push(')');
            }
            buffer.push(' ');
        }
        buffer.push_str("COLLATE ");
        AnyName(name).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for ColumnDef {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if let Some(ref name) = self.colname {
            buffer.push_str(name);
        }
        if let Some(ref name) = self.type_name {
            if !buffer.is_empty() && !buffer.ends_with(' ') {
                buffer.push(' ');
            }
            (**name).build(buffer)?;
        }
        if let Some(ref raw) = self.raw_default {
            buffer.push(' ');
            buffer.push_str("USING ");
            Expr(&**raw).build(buffer)?;
        }
        if let Some(ref fdw) = self.fdwoptions {
            buffer.push(' ');
            CreateGenericOptions(fdw).build(buffer)?;
        }
        if let Some(ref constraints) = self.constraints {
            for constraint in iter_only!(constraints, Node::Constraint) {
                buffer.push(' ');
                constraint.build(buffer)?;
            }
        }

        if let Some(ref clause) = self.coll_clause {
            buffer.push(' ');
            (**clause).build(buffer)?;
        }

        Ok(())
    }
}

impl SqlBuilder for ColumnRef {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let fields = must!(self.fields);
        let mut iter = fields.iter();
        if let Some(node) = iter.next() {
            match node {
                Node::A_Star(star) => star.build(buffer)?,
                Node::String {
                    value: Some(ref value),
                } => buffer.push_str(&quote_identifier(value)),
                _ => {}
            }
        } else {
            return Err(SqlError::Missing("fields[0]".into()));
        }

        // Do the rest via indirection. We reuse the iterator here since we want to start at the next element
        Indirection(fields, 1).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for CommentStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("COMMENT ON ");
        match *self.objtype {
            ObjectType::OBJECT_COLUMN => buffer.push_str("COLUMN "),
            ObjectType::OBJECT_INDEX => buffer.push_str("INDEX "),
            ObjectType::OBJECT_SEQUENCE => buffer.push_str("SEQUENCE "),
            ObjectType::OBJECT_STATISTIC_EXT => buffer.push_str("STATISTICS "),
            ObjectType::OBJECT_TABLE => buffer.push_str("TABLE "),
            ObjectType::OBJECT_VIEW => buffer.push_str("VIEW "),
            ObjectType::OBJECT_MATVIEW => buffer.push_str("MATERIALIZED VIEW "),
            ObjectType::OBJECT_COLLATION => buffer.push_str("COLLATION "),
            ObjectType::OBJECT_CONVERSION => buffer.push_str("CONVERSION "),
            ObjectType::OBJECT_FOREIGN_TABLE => buffer.push_str("FOREIGN TABLE "),
            ObjectType::OBJECT_TSCONFIGURATION => buffer.push_str("TEXT SEARCH CONFIGURATION "),
            ObjectType::OBJECT_TSDICTIONARY => buffer.push_str("TEXT SEARCH DICTIONARY "),
            ObjectType::OBJECT_TSPARSER => buffer.push_str("TEXT SEARCH PARSER "),
            ObjectType::OBJECT_TSTEMPLATE => buffer.push_str("TEXT SEARCH TEMPLATE "),
            ObjectType::OBJECT_ACCESS_METHOD => buffer.push_str("ACCESS METHOD "),
            ObjectType::OBJECT_DATABASE => buffer.push_str("DATABASE "),
            ObjectType::OBJECT_EVENT_TRIGGER => buffer.push_str("EVENT TRIGGER "),
            ObjectType::OBJECT_EXTENSION => buffer.push_str("EXTENSION "),
            ObjectType::OBJECT_FDW => buffer.push_str("FOREIGN DATA WRAPPER "),
            ObjectType::OBJECT_LANGUAGE => buffer.push_str("LANGUAGE "),
            ObjectType::OBJECT_PUBLICATION => buffer.push_str("PUBLICATION "),
            ObjectType::OBJECT_ROLE => buffer.push_str("ROLE "),
            ObjectType::OBJECT_SCHEMA => buffer.push_str("SCHEMA "),
            ObjectType::OBJECT_FOREIGN_SERVER => buffer.push_str("SERVER "),
            ObjectType::OBJECT_SUBSCRIPTION => buffer.push_str("SUBSCRIPTION "),
            ObjectType::OBJECT_TABLESPACE => buffer.push_str("TABLESPACE "),
            ObjectType::OBJECT_TYPE => buffer.push_str("TYPE "),
            ObjectType::OBJECT_DOMAIN => buffer.push_str("DOMAIN "),
            ObjectType::OBJECT_AGGREGATE => buffer.push_str("AGGREGATE "),
            ObjectType::OBJECT_FUNCTION => buffer.push_str("FUNCTION "),
            ObjectType::OBJECT_OPERATOR => buffer.push_str("OPERATOR "),
            ObjectType::OBJECT_TABCONSTRAINT => buffer.push_str("CONSTRAINT "),
            ObjectType::OBJECT_DOMCONSTRAINT => buffer.push_str("CONSTRAINT "),
            ObjectType::OBJECT_POLICY => buffer.push_str("POLICY "),
            ObjectType::OBJECT_PROCEDURE => buffer.push_str("PROCEDURE "),
            ObjectType::OBJECT_ROUTINE => buffer.push_str("ROUTINE "),
            ObjectType::OBJECT_RULE => buffer.push_str("RULE "),
            ObjectType::OBJECT_TRANSFORM => buffer.push_str("TRANSFORM "),
            ObjectType::OBJECT_TRIGGER => buffer.push_str("TRIGGER "),
            ObjectType::OBJECT_OPCLASS => buffer.push_str("OPERATOR CLASS "),
            ObjectType::OBJECT_OPFAMILY => buffer.push_str("OPERATOR FAMILY "),
            ObjectType::OBJECT_LARGEOBJECT => buffer.push_str("LARGE OBJECT "),
            ObjectType::OBJECT_CAST => buffer.push_str("CAST "),
            unexpected => unsupported!(unexpected),
        }

        // object parsing
        let object = must!(self.object);
        match *self.objtype {
            ObjectType::OBJECT_COLUMN
            | ObjectType::OBJECT_INDEX
            | ObjectType::OBJECT_SEQUENCE
            | ObjectType::OBJECT_STATISTIC_EXT
            | ObjectType::OBJECT_TABLE
            | ObjectType::OBJECT_VIEW
            | ObjectType::OBJECT_MATVIEW
            | ObjectType::OBJECT_COLLATION
            | ObjectType::OBJECT_CONVERSION
            | ObjectType::OBJECT_FOREIGN_TABLE
            | ObjectType::OBJECT_TSCONFIGURATION
            | ObjectType::OBJECT_TSDICTIONARY
            | ObjectType::OBJECT_TSPARSER
            | ObjectType::OBJECT_TSTEMPLATE => {
                let list = node!(**object, Node::List);
                AnyName(&list.items).build(buffer)?;
            }
            ObjectType::OBJECT_ACCESS_METHOD
            | ObjectType::OBJECT_DATABASE
            | ObjectType::OBJECT_EVENT_TRIGGER
            | ObjectType::OBJECT_EXTENSION
            | ObjectType::OBJECT_FDW
            | ObjectType::OBJECT_LANGUAGE
            | ObjectType::OBJECT_PUBLICATION
            | ObjectType::OBJECT_ROLE
            | ObjectType::OBJECT_SCHEMA
            | ObjectType::OBJECT_FOREIGN_SERVER
            | ObjectType::OBJECT_SUBSCRIPTION
            | ObjectType::OBJECT_TABLESPACE => {
                let value = string_value!(**object);
                buffer.push_str(&quote_identifier(value));
            }
            ObjectType::OBJECT_TYPE | ObjectType::OBJECT_DOMAIN => {
                let type_name = node!(**object, Node::TypeName);
                type_name.build(buffer)?;
            }
            ObjectType::OBJECT_AGGREGATE => {
                let owa = node!(**object, Node::ObjectWithArgs);
                AggregateWithArgTypes(owa).build(buffer)?;
            }
            ObjectType::OBJECT_FUNCTION
            | ObjectType::OBJECT_PROCEDURE
            | ObjectType::OBJECT_ROUTINE => {
                let owa = node!(**object, Node::ObjectWithArgs);
                FunctionWithArgTypes(owa).build(buffer)?;
            }
            ObjectType::OBJECT_OPERATOR => {
                let owa = node!(**object, Node::ObjectWithArgs);
                OperatorWithArgTypes(owa).build(buffer)?;
            }
            ObjectType::OBJECT_TABCONSTRAINT
            | ObjectType::OBJECT_POLICY
            | ObjectType::OBJECT_RULE
            | ObjectType::OBJECT_TRIGGER => {
                let list = node!(**object, Node::List);
                if list.items.len() < 2 {
                    return Err(SqlError::Unsupported("list.items.len() < 2".into()));
                }
                let last = string_value!(list.items.iter().last().unwrap());
                buffer.push_str(&quote_identifier(last));
                buffer.push_str(" ON ");
                AnyName(&list.items[0..list.items.len() - 1]).build(buffer)?;
            }
            ObjectType::OBJECT_DOMCONSTRAINT => {
                let list = node!(**object, Node::List);
                if list.items.len() < 2 {
                    return Err(SqlError::Unsupported("list.items.len() < 2".into()));
                }
                let last = string_value!(list.items.iter().last().unwrap());
                buffer.push_str(&quote_identifier(last));
                buffer.push_str(" ON DOMAIN ");
                let typ = node!(list.items[0], Node::TypeName);
                typ.build(buffer)?;
            }
            ObjectType::OBJECT_TRANSFORM => {
                let list = node!(**object, Node::List);
                if list.items.len() < 2 {
                    return Err(SqlError::Unsupported("list.items.len() < 2".into()));
                }
                buffer.push_str("FOR ");
                node!(list.items[0], Node::TypeName).build(buffer)?;
                buffer.push_str(" LANGUAGE ");
                buffer.push_str(&quote_identifier(string_value!(list.items[1])));
            }
            ObjectType::OBJECT_OPCLASS | ObjectType::OBJECT_OPFAMILY => {
                let list = node!(**object, Node::List);
                if list.items.len() < 2 {
                    return Err(SqlError::Unsupported("list.items.len() < 2".into()));
                }
                AnyName(&list.items[1..]).build(buffer)?;
                buffer.push_str(" USING ");
                buffer.push_str(&quote_identifier(string_value!(list.items[0])));
            }
            ObjectType::OBJECT_LARGEOBJECT => {
                SqlValue(&**object).build_with_context(buffer, Context::None)?;
            }
            ObjectType::OBJECT_CAST => {
                let list = node!(**object, Node::List);
                if list.items.len() < 2 {
                    return Err(SqlError::Unsupported("list.items.len() < 2".into()));
                }
                buffer.push('(');
                node!(list.items[0], Node::TypeName).build(buffer)?;
                buffer.push_str(" AS ");
                node!(list.items[1], Node::TypeName).build(buffer)?;
                buffer.push(')');
            }
            unexpected => unsupported!(unexpected),
        }

        buffer.push_str(" IS ");
        if let Some(ref comment) = self.comment {
            StringLiteral(comment).build(buffer)?;
        } else {
            buffer.push_str("NULL");
        }
        Ok(())
    }
}

impl SqlBuilder for CommonTableExpr {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.ctename);
        ColId(name).build(buffer)?;

        if let Some(ref alias_column_names) = self.aliascolnames {
            if !alias_column_names.is_empty() {
                buffer.push('(');
                NameList(alias_column_names).build(buffer)?;
                buffer.push(')');
            }
        }

        buffer.push_str(" AS ");

        // Materialized keywords
        match *self.ctematerialized {
            CTEMaterialize::CTEMaterializeDefault => {}
            CTEMaterialize::CTEMaterializeAlways => buffer.push_str("MATERIALIZED "),
            CTEMaterialize::CTEMaterializeNever => buffer.push_str("NOT MATERIALIZED "),
        }

        // Finally, the query
        buffer.push('(');
        let query = must!(self.ctequery);
        PreparableStmt(&**query).build(buffer)?;
        buffer.push(')');

        Ok(())
    }
}

impl SqlBuilder for CompositeTypeStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let type_var = must!(self.typevar);
        buffer.push_str("CREATE TYPE ");
        (**type_var).build_with_context(buffer, Context::CreateType)?;
        buffer.push_str(" AS (");
        if let Some(ref list) = self.coldeflist {
            for (index, col) in iter_only!(list, Node::ColumnDef).enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                }
                col.build(buffer)?;
            }
        }
        buffer.push(')');
        Ok(())
    }
}

impl SqlBuilder for Constraint {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if let Some(ref name) = self.conname {
            buffer.push_str("CONSTRAINT ");
            buffer.push_str(name);
            buffer.push(' ');
        }

        // Do the type
        match *self.contype {
            ConstrType::CONSTR_NULL => buffer.push_str("NULL"),
            ConstrType::CONSTR_NOTNULL => buffer.push_str("NOT NULL"),
            ConstrType::CONSTR_DEFAULT => {
                buffer.push_str("DEFAULT");
                if let Some(ref raw) = self.raw_expr {
                    buffer.push(' ');
                    Expr(&**raw).build(buffer)?;
                }
            }
            ConstrType::CONSTR_IDENTITY => {
                buffer.push_str("GENERATED ");
                match self.generated_when {
                    constants::ATTRIBUTE_IDENTITY_ALWAYS => buffer.push_str("ALWAYS "),
                    constants::ATTRIBUTE_IDENTITY_BY_DEFAULT => buffer.push_str("BY DEFAULT "),
                    unexpected => {
                        return Err(SqlError::Unsupported(format!(
                            "Unexpected attribute identity: {}",
                            unexpected
                        )))
                    }
                }
                buffer.push_str("AS IDENTITY");
                if let Some(ref options) = self.options {
                    buffer.push(' ');
                    ParenthesizedSeqOptList(options).build(buffer)?;
                }
            }
            ConstrType::CONSTR_GENERATED => {
                if self.generated_when != constants::ATTRIBUTE_IDENTITY_ALWAYS {
                    return Err(SqlError::Unsupported(format!(
                        "Unexpected attribute identity: {}",
                        self.generated_when
                    )));
                }
                buffer.push_str("GENERATED ALWAYS AS (");
                if let Some(ref raw) = self.raw_expr {
                    Expr(&**raw).build(buffer)?;
                }
                buffer.push_str(") STORED");
            }
            ConstrType::CONSTR_CHECK => {
                buffer.push_str("CHECK (");
                if let Some(ref raw) = self.raw_expr {
                    Expr(&**raw).build(buffer)?;
                }
                buffer.push(')');
            }
            ConstrType::CONSTR_PRIMARY => buffer.push_str("PRIMARY KEY"),
            ConstrType::CONSTR_UNIQUE => buffer.push_str("UNIQUE"),
            ConstrType::CONSTR_EXCLUSION => {
                buffer.push_str("EXCLUDE ");
                if let Some(ref method) = self.access_method {
                    if method.ne(constants::DEFAULT_INDEX_TYPE) {
                        buffer.push_str("USING ");
                        buffer.push_str(&quote_identifier(method));
                        buffer.push(' ');
                    }
                }
                buffer.push('(');
                if let Some(ref exclusions) = self.exclusions {
                    for (index, exclusion) in iter_only!(exclusions, Node::List).enumerate() {
                        if index > 0 {
                            buffer.push_str(", ");
                        }
                        if exclusion.items.len() != 2 {
                            return Err(SqlError::Unsupported("exclusion.items.len() != 2".into()));
                        }

                        // Parse an IndexElem from the first item
                        node!(exclusion.items[0], Node::IndexElem).build(buffer)?;
                        buffer.push_str(" WITH ");

                        // A List for the second element
                        let list = node!(exclusion.items[1], Node::List);
                        AnyOperator(&list.items).build(buffer)?;
                    }
                }
                buffer.push(')');
                if let Some(ref where_clause) = self.where_clause {
                    // Don't use a WhereClause for this - handle it here.
                    buffer.push_str(" WHERE (");
                    Expr(&**where_clause).build(buffer)?;
                    buffer.push(')');
                }
            }
            ConstrType::CONSTR_FOREIGN => {
                if let Some(ref attrs) = self.fk_attrs {
                    if !attrs.is_empty() {
                        buffer.push_str("FOREIGN KEY");
                    }
                }
            }
            ConstrType::CONSTR_ATTR_DEFERRABLE => buffer.push_str("DEFERRABLE"),
            ConstrType::CONSTR_ATTR_NOT_DEFERRABLE => buffer.push_str("NOT DEFERRABLE"),
            ConstrType::CONSTR_ATTR_DEFERRED => buffer.push_str("INITIALLY DEFERRED"),
            ConstrType::CONSTR_ATTR_IMMEDIATE => buffer.push_str("INITIALLY IMMEDIATE"),
        }

        // Key columns
        if let Some(ref keys) = self.keys {
            if !keys.is_empty() {
                buffer.push_str(" (");
                ColumnList(keys).build(buffer)?;
                buffer.push(')');
            }
        }

        // FK Attribute columns
        if let Some(ref attrs) = self.fk_attrs {
            if !attrs.is_empty() {
                buffer.push_str(" (");
                ColumnList(attrs).build(buffer)?;
                buffer.push(')');
            }
        }

        // Primary key table
        if let Some(ref table) = self.pktable {
            buffer.push_str(" REFERENCES ");
            (**table).build_with_context(buffer, Context::None)?;
            if let Some(ref attrs) = self.pk_attrs {
                if !attrs.is_empty() {
                    buffer.push_str(" (");
                    ColumnList(attrs).build(buffer)?;
                    buffer.push(')');
                }
            }
        }

        // Index match types
        match self.fk_matchtype {
            constants::FKCONSTR_MATCH_SIMPLE => {} // Default
            constants::FKCONSTR_MATCH_FULL => buffer.push_str(" MATCH FULL"),
            constants::FKCONSTR_MATCH_PARTIAL => {
                return Err(SqlError::Unsupported("Not implemented in Postgres".into()))
            }
            _ => {} // Not specified
        }

        // Update action
        match self.fk_upd_action {
            constants::FKCONSTR_ACTION_NOACTION => {} // Default
            constants::FKCONSTR_ACTION_RESTRICT => buffer.push_str(" ON UPDATE RESTRICT"),
            constants::FKCONSTR_ACTION_CASCADE => buffer.push_str(" ON UPDATE CASCADE"),
            constants::FKCONSTR_ACTION_SETNULL => buffer.push_str(" ON UPDATE SET NULL"),
            constants::FKCONSTR_ACTION_SETDEFAULT => buffer.push_str(" ON UPDATE SET DEFAULT"),
            _ => {} // Not specified
        }

        // Delete action
        match self.fk_del_action {
            constants::FKCONSTR_ACTION_NOACTION => {} // Default
            constants::FKCONSTR_ACTION_RESTRICT => buffer.push_str(" ON DELETE RESTRICT"),
            constants::FKCONSTR_ACTION_CASCADE => buffer.push_str(" ON DELETE CASCADE"),
            constants::FKCONSTR_ACTION_SETNULL => buffer.push_str(" ON DELETE SET NULL"),
            constants::FKCONSTR_ACTION_SETDEFAULT => buffer.push_str(" ON DELETE SET DEFAULT"),
            _ => {} // Not specified
        }

        // Includes
        if let Some(ref includes) = self.including {
            if !includes.is_empty() {
                buffer.push_str(" INCLUDE (");
                ColumnList(includes).build(buffer)?;
                buffer.push(')');
            }
        }

        // Index options
        if let Some(ref name) = self.indexname {
            buffer.push_str(" USING INDEX ");
            buffer.push_str(&quote_identifier(name));
        }
        if let Some(ref space) = self.indexspace {
            buffer.push_str(" USING INDEX TABLESPACE ");
            buffer.push_str(&quote_identifier(space));
        }
        if self.deferrable {
            buffer.push_str(" DEFERRABLE");
        }
        if self.initdeferred {
            buffer.push_str(" INITIALLY DEFERRED");
        }
        if self.is_no_inherit {
            buffer.push_str(" NO INHERIT");
        }
        if self.skip_validation {
            buffer.push_str(" NOT VALID");
        }
        Ok(())
    }
}

impl SqlBuilder for CopyStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        fn default_option_impl(
            inner_buffer: &mut String,
            element: &DefElem,
            other: &str,
        ) -> core::result::Result<(), SqlError> {
            inner_buffer.push_str(other);
            if let Some(ref arg) = element.arg {
                inner_buffer.push(' ');
                match &**arg {
                    Node::String {
                        value: Some(ref value),
                    } => BooleanOrString(value).build(inner_buffer)?,
                    Node::Integer { .. } | Node::Float { .. } => {
                        NumericOnly(&**arg).build(inner_buffer)?
                    }
                    Node::A_Star(a_star) => a_star.build(inner_buffer)?,
                    Node::List(list) => {
                        inner_buffer.push('(');
                        for (sub, item) in list.items.iter().enumerate() {
                            if sub > 0 {
                                inner_buffer.push_str(", ");
                            }
                            BooleanOrString(string_value!(item)).build(inner_buffer)?;
                        }
                        inner_buffer.push(')');
                    }
                    unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
                }
            }
            Ok(())
        }

        buffer.push_str("COPY");
        if let Some(ref relation) = self.relation {
            buffer.push(' ');
            (**relation).build_with_context(buffer, Context::None)?;
            if let Some(ref attlist) = self.attlist {
                if !attlist.is_empty() {
                    buffer.push('(');
                    ColumnList(attlist).build(buffer)?;
                    buffer.push(')');
                }
            }
        }

        if let Some(ref query) = self.query {
            buffer.push_str(" (");
            PreparableStmt(&**query).build(buffer)?;
            buffer.push(')');
        }

        if self.is_from {
            buffer.push_str(" FROM");
        } else {
            buffer.push_str(" TO");
        }

        if self.is_program {
            buffer.push_str(" PROGRAM");
        }

        if let Some(ref filename) = self.filename {
            buffer.push(' ');
            StringLiteral(filename).build(buffer)?;
        } else if self.is_from {
            buffer.push_str(" STDIN");
        } else {
            buffer.push_str(" STDOUT");
        }

        if let Some(ref options) = self.options {
            if !options.is_empty() {
                buffer.push_str(" WITH (");
                for (index, element) in iter_only!(options, Node::DefElem).enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    let name = must!(element.defname);

                    match &name[..] {
                        "format" => {
                            buffer.push_str("FORMAT ");
                            let format = must!(element.arg);
                            let format = string_value!(**format);
                            match &format[..] {
                                "binary" => buffer.push_str("BINARY"),
                                "csv" => buffer.push_str("CSV"),
                                unexpected => {
                                    return Err(SqlError::Unsupported(format!(
                                        "Format type: {}",
                                        unexpected
                                    )))
                                }
                            }
                        }
                        "freeze" | "header" => {
                            let mut use_default_impl = false;
                            let mut value = None;
                            if let Some(arg) = &element.arg {
                                let val = int_value!(**arg);
                                if val == 1 {
                                    value = Some(val);
                                } else {
                                    use_default_impl = true;
                                }
                            }

                            if use_default_impl {
                                default_option_impl(buffer, element, name)?;
                            } else {
                                buffer.push_str(&name.to_uppercase());
                                if let Some(val) = value {
                                    buffer.push_str(&format!(" {}", val));
                                }
                            }
                        }
                        "delimiter" => {
                            let arg = must!(element.arg);
                            buffer.push_str("DELIMITER ");
                            StringLiteral(string_value!(**arg)).build(buffer)?;
                        }
                        "null" => {
                            let arg = must!(element.arg);
                            buffer.push_str("NULL ");
                            StringLiteral(string_value!(**arg)).build(buffer)?;
                        }
                        "quote" => {
                            let arg = must!(element.arg);
                            buffer.push_str("QUOTE ");
                            StringLiteral(string_value!(**arg)).build(buffer)?;
                        }
                        "escape" => {
                            let arg = must!(element.arg);
                            buffer.push_str("ESCAPE ");
                            StringLiteral(string_value!(**arg)).build(buffer)?;
                        }
                        "encoding" => {
                            let arg = must!(element.arg);
                            buffer.push_str("ENCODING ");
                            StringLiteral(string_value!(**arg)).build(buffer)?;
                        }
                        "force_quote" => {
                            let arg = must!(element.arg);
                            buffer.push_str("FORCE_QUOTE ");
                            match &**arg {
                                Node::A_Star(a_star) => a_star.build(buffer)?,
                                Node::List(list) => {
                                    buffer.push('(');
                                    ColumnList(&list.items).build(buffer)?;
                                    buffer.push(')');
                                }
                                unexpected => {
                                    return Err(SqlError::UnexpectedNodeType(unexpected.name()))
                                }
                            }
                        }
                        "force_not_null" => {
                            let arg = must!(element.arg);
                            let list = node!(**arg, Node::List);
                            buffer.push_str("FORCE_NOT_NULL (");
                            ColumnList(&list.items).build(buffer)?;
                            buffer.push(')');
                        }
                        "force_null" => {
                            let arg = must!(element.arg);
                            let list = node!(**arg, Node::List);
                            buffer.push_str("FORCE_NULL (");
                            ColumnList(&list.items).build(buffer)?;
                            buffer.push(')');
                        }
                        other => default_option_impl(buffer, element, other)?,
                    }
                }
                buffer.push(')');
            }
        }
        if let Some(ref where_clause) = self.where_clause {
            WhereClause(where_clause).build(buffer)?;
        }
        Ok(())
    }
}

impl SqlBuilder for CreateCastStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let source_type = must!(self.sourcetype);
        let target_type = must!(self.targettype);
        buffer.push_str("CREATE CAST (");
        (**source_type).build(buffer)?;
        buffer.push_str(" AS ");
        (**target_type).build(buffer)?;
        buffer.push(')');

        // Function
        if let Some(ref func) = self.func {
            buffer.push_str(" WITH FUNCTION ");
            FunctionWithArgTypes(&**func).build(buffer)?;
        } else if self.inout {
            buffer.push_str(" WITH INOUT");
        } else {
            buffer.push_str(" WITHOUT FUNCTION");
        }

        // Context
        match *self.context {
            CoercionContext::COERCION_IMPLICIT => buffer.push_str(" AS IMPLICIT"),
            CoercionContext::COERCION_ASSIGNMENT => buffer.push_str(" AS ASSIGNMENT"),
            CoercionContext::COERCION_EXPLICIT => {}
        }

        Ok(())
    }
}

impl SqlBuilder for CreateDomainStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let type_name = must!(self.type_name);

        buffer.push_str("CREATE DOMAIN");
        if let Some(ref domain_name) = self.domainname {
            buffer.push(' ');
            AnyName(domain_name).build(buffer)?;
        }
        buffer.push_str(" AS ");
        (**type_name).build(buffer)?;

        if let Some(ref coll) = self.coll_clause {
            buffer.push(' ');
            (**coll).build(buffer)?;
        }

        if let Some(ref constraints) = self.constraints {
            for constraint in iter_only!(constraints, Node::Constraint) {
                buffer.push(' ');
                constraint.build(buffer)?;
            }
        }
        Ok(())
    }
}

impl SqlBuilder for CreateEnumStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let type_name = must!(self.type_name);
        buffer.push_str("CREATE TYPE ");
        AnyName(type_name).build(buffer)?;
        buffer.push_str(" AS ENUM (");
        if let Some(ref values) = self.vals {
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                }
                StringLiteral(string_value!(value)).build(buffer)?;
            }
        }
        buffer.push(')');
        Ok(())
    }
}

impl SqlBuilder for CreateExtensionStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let extname = must!(self.extname);

        buffer.push_str("CREATE EXTENSION ");
        if self.if_not_exists {
            buffer.push_str("IF NOT EXISTS ");
        }
        ColId(extname).build(buffer)?;

        if let Some(ref options) = self.options {
            for elem in iter_only!(options, Node::DefElem) {
                let name = must!(elem.defname);

                match &name[..] {
                    "schema" => {
                        let arg = must!(elem.arg);
                        buffer.push_str(" SCHEMA ");
                        ColId(string_value!(**arg)).build(buffer)?;
                    }
                    "new_version" => {
                        let arg = must!(elem.arg);
                        buffer.push_str(" VERSION ");
                        NonReservedWordOrSconst(&**arg).build(buffer)?;
                    }
                    "cascade" => buffer.push_str(" CASCADE"),
                    unexpected => {
                        return Err(SqlError::Unsupported(format!(
                            "Extension option: {}",
                            unexpected
                        )))
                    }
                }
            }
        }
        Ok(())
    }
}

impl SqlBuilder for CreateFunctionStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.funcname);

        buffer.push_str("CREATE ");
        if self.replace {
            buffer.push_str("OR REPLACE ");
        }
        if self.is_procedure {
            buffer.push_str("PROCEDURE ");
        } else {
            buffer.push_str("FUNCTION ");
        }

        // Function name
        FuncName(name).build(buffer)?;

        // Parameters
        let mut tables = Vec::new();
        buffer.push('(');
        if let Some(ref parameters) = self.parameters {
            let mut comma = false;
            for parameter in iter_only!(parameters, Node::FunctionParameter) {
                if *parameter.mode != FunctionParameterMode::FUNC_PARAM_TABLE {
                    if comma {
                        buffer.push_str(", ");
                    } else {
                        comma = true;
                    }
                    parameter.build(buffer)?;
                } else {
                    tables.push(parameter);
                }
            }
        }
        buffer.push(')');

        // If it's a table func then handle it
        if !tables.is_empty() {
            buffer.push_str(" RETURNS TABLE (");
            for (index, table) in tables.iter().enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                }
                table.build(buffer)?;
            }
            buffer.push(')');
        } else if let Some(ref return_type) = self.return_type {
            buffer.push_str(" RETURNS ");
            (**return_type).build(buffer)?;
        }

        // Finally, options
        if let Some(ref options) = self.options {
            for option in iter_only!(options, Node::DefElem) {
                buffer.push(' ');

                // "createfunc_opt_item" in gram.y
                let name = must!(option.defname);

                if name.eq_ignore_ascii_case("AS") {
                    buffer.push_str("AS ");

                    // "func_as" in gram.y
                    let arg = must!(option.arg);
                    let list = node!(**arg, Node::List);
                    let list = node_vec_to_string_vec(&list.items);
                    for (index, item) in list.iter().enumerate() {
                        if index > 0 {
                            buffer.push_str(", ");
                        }
                        if !item.contains("$$") {
                            buffer.push_str("$$");
                            buffer.push_str(item);
                            buffer.push_str("$$");
                        } else {
                            StringLiteral(item).build(buffer)?;
                        }
                    }
                } else if name.eq_ignore_ascii_case("LANGUAGE") {
                    let arg = must!(option.arg);
                    buffer.push_str("LANGUAGE ");
                    NonReservedWordOrSconst(&**arg).build(buffer)?;
                } else if name.eq_ignore_ascii_case("transform") {
                    let arg = must!(option.arg);
                    let list = node!(**arg, Node::List);
                    buffer.push_str("TRANSFORM ");
                    for (index, item) in list.items.iter().enumerate() {
                        if index > 0 {
                            buffer.push_str(", ");
                        }
                        buffer.push_str("FOR TYPE ");
                        node!(item, Node::TypeName).build(buffer)?;
                    }
                } else if name.eq_ignore_ascii_case("WINDOW") {
                    buffer.push_str("WINDOW");
                } else {
                    CommonFuncOptItem(option).build(buffer)?;
                }
            }
        }
        Ok(())
    }
}

impl SqlBuilder for CreateRangeStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let type_name = must!(self.type_name);
        let definition = must!(self.params);
        buffer.push_str("CREATE TYPE ");
        AnyName(type_name).build(buffer)?;
        buffer.push_str(" AS RANGE ");
        Definition(definition).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for CreateSchemaStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("CREATE SCHEMA");
        if self.if_not_exists {
            buffer.push_str(" IF NOT EXISTS");
        }
        if let Some(ref name) = self.schemaname {
            buffer.push(' ');
            ColId(name).build(buffer)?;
        }
        if let Some(ref role) = self.authrole {
            buffer.push_str(" AUTHORIZATION ");
            (**role).build(buffer)?;
        }
        if let Some(ref elements) = self.schema_elts {
            for el in elements.iter() {
                buffer.push(' ');

                SchemaStmt(el).build(buffer)?;
            }
        }
        Ok(())
    }
}

impl SqlBuilder for CreateSeqStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let sequence = must!(self.sequence);

        buffer.push_str("CREATE ");

        if let Some(temp) = persistence_from_code(sequence.relpersistence) {
            buffer.push(' ');
            buffer.push_str(temp);
        }

        buffer.push_str(" SEQUENCE");
        if self.if_not_exists {
            buffer.push_str(" IF NOT EXISTS");
        }
        (**sequence).build_with_context(buffer, Context::None)?;
        if let Some(ref options) = self.options {
            if !options.is_empty() {
                buffer.push(' ');
                SeqOptList(options).build(buffer)?;
            }
        }
        Ok(())
    }
}

impl SqlBuilderWithContext for CreateStmt {
    fn build_with_context(
        &self,
        buffer: &mut String,
        context: Context,
    ) -> core::result::Result<(), SqlError> {
        let relation = must!(self.relation);

        buffer.push_str("CREATE ");

        if context == Context::ForeignTable {
            buffer.push_str("FOREIGN ");
        }

        // Temp table
        if let Some(persistence) = persistence_from_code(relation.relpersistence) {
            buffer.push_str(persistence);
            buffer.push(' ');
        }

        buffer.push_str("TABLE ");
        if self.if_not_exists {
            buffer.push_str("IF NOT EXISTS ");
        }

        (**relation).build_with_context(buffer, Context::None)?;

        // OF type
        if let Some(ref type_name) = self.of_typename {
            buffer.push_str(" OF ");
            (**type_name).build(buffer)?;
        }

        // Partition
        if self.partbound.is_some() {
            if let Some(ref inh) = self.inh_relations {
                if let Some(Node::RangeVar(ref range)) = inh.iter().next() {
                    buffer.push_str(" PARTITION OF ");
                    range.build_with_context(buffer, Context::None)?;
                }
                // else, error?
            }
        }

        if let Some(ref table_elements) = self.table_elts {
            if !table_elements.is_empty() {
                buffer.push_str(" (");
                for (index, item) in table_elements.iter().enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    // Could restrict to "TableElement" in gram.y
                    item.build(buffer)?;
                }
                buffer.push(')')
            } else if self.partbound.is_none() && self.of_typename.is_none() {
                buffer.push_str(" ()");
            }
        } else if self.partbound.is_none() && self.of_typename.is_none() {
            buffer.push_str(" ()");
        }

        if let Some(ref bound) = self.partbound {
            buffer.push(' ');
            (**bound).build(buffer)?;
        } else if let Some(ref inh) = self.inh_relations {
            if !inh.is_empty() {
                buffer.push_str(" INHERITS (");
                QualifiedNameList(inh).build(buffer)?;
                buffer.push(')');
            }
        }

        if let Some(ref spec) = self.partspec {
            buffer.push(' ');
            (**spec).build(buffer)?;
        }

        if let Some(ref access_method) = self.access_method {
            buffer.push_str(" USING ");
            buffer.push_str(&quote_identifier(access_method));
        }

        if let Some(ref options) = self.options {
            OptWith(options).build(buffer)?;
        }

        match *self.oncommit {
            OnCommitAction::ONCOMMIT_NOOP => {} // No ON COMMIT clause
            OnCommitAction::ONCOMMIT_PRESERVE_ROWS => {
                buffer.push_str(" ON COMMIT PRESERVE ROWS");
            }
            OnCommitAction::ONCOMMIT_DELETE_ROWS => {
                buffer.push_str(" ON COMMIT DELETE ROWS");
            }
            OnCommitAction::ONCOMMIT_DROP => {
                buffer.push_str(" ON COMMIT DROP");
            }
        }

        if let Some(ref tablespace_name) = self.tablespacename {
            buffer.push_str(" TABLESPACE ");
            buffer.push_str(&quote_identifier(tablespace_name));
        }
        Ok(())
    }
}

impl SqlBuilder for CreateTableAsStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let into = must!(self.into);

        buffer.push_str("CREATE");

        if let Some(ref rel) = into.rel {
            if let Some(persistence) = persistence_from_code(rel.relpersistence) {
                buffer.push(' ');
                buffer.push_str(persistence);
            }
        }

        // Relation kind
        match *self.relkind {
            ObjectType::OBJECT_TABLE => buffer.push_str(" TABLE"),
            ObjectType::OBJECT_MATVIEW => buffer.push_str(" MATERIALIZED VIEW"),
            // Unsupported here
            unsupported => return Err(SqlError::Unsupported(format!("{:?}", unsupported))),
        }

        if self.if_not_exists {
            buffer.push_str(" IF NOT EXISTS");
        }

        buffer.push(' ');
        (**into).build(buffer)?;
        buffer.push_str(" AS ");

        if let Some(ref query) = self.query {
            match &**query {
                Node::ExecuteStmt(stmt) => stmt.build(buffer)?,
                Node::SelectStmt(stmt) => stmt.build(buffer)?,
                unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
            }
        }
        if into.skip_data {
            buffer.push_str(" WITH NO DATA");
        }
        Ok(())
    }
}

impl SqlBuilder for CreateTableSpaceStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.tablespacename);
        let location = must!(self.location);

        buffer.push_str("CREATE TABLESPACE ");
        ColId(name).build(buffer)?;

        if let Some(ref owner) = self.owner {
            buffer.push_str(" OWNER ");
            (**owner).build(buffer)?;
        }

        buffer.push_str(" LOCATION ");
        StringLiteral(location).build(buffer)?;

        if let Some(ref options) = self.options {
            buffer.push(' ');
            OptWith(options).build(buffer)?;
        }
        Ok(())
    }
}

impl SqlBuilder for CreateTrigStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.trigname);
        let func_name = must!(self.funcname);
        let relation = must!(self.relation);

        buffer.push_str("CREATE");
        if self.isconstraint {
            buffer.push_str(" CONSTRAINT");
        }
        buffer.push_str(" TRIGGER");
        buffer.push_str(name);
        buffer.push(' ');

        match self.timing {
            constants::trigger::TRIGGER_TYPE_BEFORE => buffer.push_str(" BEFORE"),
            constants::trigger::TRIGGER_TYPE_AFTER => buffer.push_str(" AFTER"),
            constants::trigger::TRIGGER_TYPE_INSTEAD => buffer.push_str(" INSTEAD"),
            unknown => return Err(SqlError::Unsupported(format!("Timing: {}", unknown))),
        }

        let mut require_or = false;
        if self.events & constants::trigger::TRIGGER_TYPE_INSERT > 0 {
            buffer.push_str(" INSERT");
            require_or = true;
        }
        if self.events & constants::trigger::TRIGGER_TYPE_DELETE > 0 {
            if require_or {
                buffer.push_str(" OR");
            }
            buffer.push_str(" DELETE");
            require_or = true;
        }
        if self.events & constants::trigger::TRIGGER_TYPE_UPDATE > 0 {
            if require_or {
                buffer.push_str(" OR");
            }
            buffer.push_str(" UPDATE");
            require_or = true;
            if let Some(ref columns) = self.columns {
                if !columns.is_empty() {
                    buffer.push_str(" OF");
                    ColumnList(columns).build(buffer)?;
                }
            }
        }
        if self.events & constants::trigger::TRIGGER_TYPE_TRUNCATE > 0 {
            if require_or {
                buffer.push_str(" OR");
            }
            buffer.push_str(" TRUNCATE");
        }

        buffer.push_str(" ON");
        (**relation).build_with_context(buffer, Context::None)?;

        if let Some(ref transitions) = self.transition_rels {
            buffer.push_str(" REFERENCING");
            for transition in iter_only!(transitions, Node::TriggerTransition) {
                buffer.push(' ');
                transition.build(buffer)?;
            }
        }

        if let Some(ref constraint) = self.constrrel {
            buffer.push_str(" FROM");
            (**constraint).build_with_context(buffer, Context::None)?;
        }

        if self.deferrable {
            buffer.push_str(" DEFERRABLE");
        }

        if self.initdeferred {
            buffer.push_str(" INITIALLY DEFERRED");
        }

        if self.row {
            buffer.push_str(" FOR EACH ROW");
        }

        if let Some(ref when_clause) = self.when_clause {
            buffer.push_str(" WHEN (");
            Expr(&**when_clause).build(buffer)?;
            buffer.push(')');
        }

        buffer.push_str(" EXECUTE FUNCTION ");
        FuncName(func_name).build(buffer)?;
        buffer.push('(');
        if let Some(ref args) = self.args {
            for (index, arg) in node_vec_to_string_vec(args).iter().enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                }
                StringLiteral(arg).build(buffer)?;
            }
        }
        buffer.push(')');
        Ok(())
    }
}

impl SqlBuilder for CreatedbStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let dbname = must!(self.dbname);
        buffer.push_str("CREATE DATABASE ");
        ColId(dbname).build(buffer)?;
        if let Some(ref options) = self.options {
            if !options.is_empty() {
                buffer.push(' ');
                CreatedbOptList(options).build(buffer)?;
            }
        }
        Ok(())
    }
}

impl SqlBuilder for DefineStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("CREATE");
        if self.replace {
            buffer.push_str(" OR REPLACE")
        }

        match *self.kind {
            ObjectType::OBJECT_AGGREGATE => buffer.push_str(" AGGREGATE"),
            ObjectType::OBJECT_OPERATOR => buffer.push_str(" OPERATOR"),
            ObjectType::OBJECT_TYPE => buffer.push_str(" TYPE"),
            ObjectType::OBJECT_TSPARSER => buffer.push_str(" TEXT SEARCH PARSER"),
            ObjectType::OBJECT_TSDICTIONARY => buffer.push_str(" TEXT SEARCH DICTIONARY"),
            ObjectType::OBJECT_TSTEMPLATE => buffer.push_str(" TEXT SEARCH TEMPLATE"),
            ObjectType::OBJECT_TSCONFIGURATION => buffer.push_str(" TEXT SEARCH CONFIGURATION"),
            ObjectType::OBJECT_COLLATION => buffer.push_str(" COLLATION"),
            unexpected => unsupported!(unexpected),
        }

        if self.if_not_exists {
            buffer.push_str(" IF NOT EXISTS");
        }

        match *self.kind {
            ObjectType::OBJECT_AGGREGATE => {
                let name = must!(self.defnames);
                buffer.push(' ');
                FuncName(name).build(buffer)?;
            }
            ObjectType::OBJECT_OPERATOR => {
                let name = must!(self.defnames);
                buffer.push(' ');
                AnyOperator(name).build(buffer)?;
            }
            ObjectType::OBJECT_TYPE
            | ObjectType::OBJECT_TSPARSER
            | ObjectType::OBJECT_TSTEMPLATE
            | ObjectType::OBJECT_TSCONFIGURATION
            | ObjectType::OBJECT_TSDICTIONARY
            | ObjectType::OBJECT_COLLATION => {
                let name = must!(self.defnames);
                buffer.push(' ');
                AnyName(name).build(buffer)?;
            }
            unexpected => unsupported!(unexpected),
        }

        if !self.oldstyle && *self.kind == ObjectType::OBJECT_AGGREGATE {
            buffer.push(' ');
            let args = must!(self.args);
            AggrArgs(args).build(buffer)?;
        }

        if let Some(ref definition) = self.definition {
            if *self.kind == ObjectType::OBJECT_COLLATION && definition.len() == 1 {
                let elem = node!(definition[0], Node::DefElem);
                let name = must!(elem.defname);
                if name.eq("from") {
                    buffer.push_str(" FROM ");
                    let arg = must!(elem.arg);
                    let arg = node!(**arg, Node::List);
                    AnyName(&arg.items).build(buffer)?;
                } else {
                    buffer.push(' ');
                    Definition(definition).build(buffer)?;
                }
            } else if !definition.is_empty() {
                buffer.push(' ');
                Definition(definition).build(buffer)?;
            }
        }

        Ok(())
    }
}

impl SqlBuilder for DeleteStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let relation = must!(self.relation);

        if let Some(ref with) = self.with_clause {
            (**with).build(buffer)?;
            buffer.push(' ');
        }

        buffer.push_str("DELETE FROM ");
        (**relation).build_with_context(buffer, Context::None)?;

        if let Some(ref using) = self.using_clause {
            buffer.push_str(" USING ");
            FromList(using).build(buffer)?;
        }

        if let Some(ref clause) = self.where_clause {
            buffer.push(' ');
            WhereClause(&**clause).build(buffer)?;
        }

        if let Some(ref list) = self.returning_list {
            if !list.is_empty() {
                buffer.push_str(" RETURNING ");
                TargetList(list).build(buffer)?;
            }
        }

        Ok(())
    }
}

impl SqlBuilder for DiscardStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("DISCARD ");
        match *self.target {
            DiscardMode::DISCARD_ALL => buffer.push_str("ALL"),
            DiscardMode::DISCARD_PLANS => buffer.push_str("PLANS"),
            DiscardMode::DISCARD_SEQUENCES => buffer.push_str("SEQUENCES"),
            DiscardMode::DISCARD_TEMP => buffer.push_str("TEMP"),
        }
        Ok(())
    }
}

impl SqlBuilder for DoStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("DO");
        if let Some(ref args) = self.args {
            for arg in iter_only!(args, Node::DefElem) {
                if let Some(ref name) = arg.defname {
                    if name.eq("language") {
                        let arg = must!(arg.arg);
                        buffer.push_str(" LANGUAGE ");
                        buffer.push_str(&quote_identifier(string_value!(**arg)));
                    } else if name.eq("as") {
                        let arg = must!(arg.arg);
                        let arg = string_value!(**arg);
                        let delim = if arg.contains("$$") { "$outer$" } else { "$$" };
                        buffer.push(' ');
                        buffer.push_str(delim);
                        buffer.push_str(arg);
                        buffer.push_str(delim);
                    }
                }
            }
        }
        Ok(())
    }
}

impl SqlBuilder for DropRoleStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let roles = must!(self.roles);
        buffer.push_str("DROP ROLE ");
        if self.missing_ok {
            buffer.push_str("IF EXISTS ");
        }
        RoleList(roles).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for DropStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let objects = must!(self.objects);

        buffer.push_str("DROP");
        match *self.remove_type {
            ObjectType::OBJECT_ACCESS_METHOD => buffer.push_str(" ACCESS METHOD"),
            ObjectType::OBJECT_AGGREGATE => buffer.push_str(" AGGREGATE"),
            ObjectType::OBJECT_CAST => buffer.push_str(" CAST"),
            ObjectType::OBJECT_COLLATION => buffer.push_str(" COLLATION"),
            ObjectType::OBJECT_CONVERSION => buffer.push_str(" CONVERSION"),
            ObjectType::OBJECT_DOMAIN => buffer.push_str(" DOMAIN"),
            ObjectType::OBJECT_EVENT_TRIGGER => buffer.push_str(" EVENT TRIGGER"),
            ObjectType::OBJECT_EXTENSION => buffer.push_str(" EXTENSION"),
            ObjectType::OBJECT_FDW => buffer.push_str(" FOREIGN DATA WRAPPER"),
            ObjectType::OBJECT_FOREIGN_SERVER => buffer.push_str(" SERVER"),
            ObjectType::OBJECT_FOREIGN_TABLE => buffer.push_str(" FOREIGN TABLE"),
            ObjectType::OBJECT_FUNCTION => buffer.push_str(" FUNCTION"),
            ObjectType::OBJECT_INDEX => buffer.push_str(" INDEX"),
            ObjectType::OBJECT_LANGUAGE => buffer.push_str(" LANGUAGE"),
            ObjectType::OBJECT_MATVIEW => buffer.push_str(" MATERIALIZED VIEW"),
            ObjectType::OBJECT_OPCLASS => buffer.push_str(" OPERATOR CLASS"),
            ObjectType::OBJECT_OPERATOR => buffer.push_str(" OPERATOR"),
            ObjectType::OBJECT_OPFAMILY => buffer.push_str(" OPERATOR FAMILY"),
            ObjectType::OBJECT_POLICY => buffer.push_str(" POLICY"),
            ObjectType::OBJECT_PROCEDURE => buffer.push_str(" PROCEDURE"),
            ObjectType::OBJECT_PUBLICATION => buffer.push_str(" PUBLICATION"),
            ObjectType::OBJECT_ROUTINE => buffer.push_str(" ROUTINE"),
            ObjectType::OBJECT_RULE => buffer.push_str(" RULE"),
            ObjectType::OBJECT_SCHEMA => buffer.push_str(" SCHEMA"),
            ObjectType::OBJECT_SEQUENCE => buffer.push_str(" SEQUENCE"),
            ObjectType::OBJECT_STATISTIC_EXT => buffer.push_str(" STATISTICS"),
            ObjectType::OBJECT_TABLE => buffer.push_str(" TABLE"),
            ObjectType::OBJECT_TRANSFORM => buffer.push_str(" TRANSFORM"),
            ObjectType::OBJECT_TRIGGER => buffer.push_str(" TRIGGER"),
            ObjectType::OBJECT_TSCONFIGURATION => buffer.push_str(" TEXT SEARCH CONFIGURATION"),
            ObjectType::OBJECT_TSDICTIONARY => buffer.push_str(" TEXT SEARCH DICTIONARY"),
            ObjectType::OBJECT_TSPARSER => buffer.push_str(" TEXT SEARCH PARSER"),
            ObjectType::OBJECT_TSTEMPLATE => buffer.push_str(" TEXT SEARCH TEMPLATE"),
            ObjectType::OBJECT_TYPE => buffer.push_str(" TYPE"),
            ObjectType::OBJECT_VIEW => buffer.push_str(" VIEW"),
            unsupported => return Err(SqlError::Unsupported(format!("{:?}", unsupported))),
        }

        if self.concurrent {
            buffer.push_str(" CONCURRENTLY");
        }
        if self.missing_ok {
            buffer.push_str(" IF EXISTS");
        }

        match *self.remove_type {
            // drop_type_any_name
            ObjectType::OBJECT_TABLE
            | ObjectType::OBJECT_SEQUENCE
            | ObjectType::OBJECT_VIEW
            | ObjectType::OBJECT_MATVIEW
            | ObjectType::OBJECT_INDEX
            | ObjectType::OBJECT_FOREIGN_TABLE
            | ObjectType::OBJECT_COLLATION
            | ObjectType::OBJECT_CONVERSION
            | ObjectType::OBJECT_STATISTIC_EXT
            | ObjectType::OBJECT_TSPARSER
            | ObjectType::OBJECT_TSDICTIONARY
            | ObjectType::OBJECT_TSTEMPLATE
            | ObjectType::OBJECT_TSCONFIGURATION => {
                buffer.push(' ');
                AnyNameList(objects).build(buffer)?;
            }
            // drop_type_name
            ObjectType::OBJECT_ACCESS_METHOD
            | ObjectType::OBJECT_EVENT_TRIGGER
            | ObjectType::OBJECT_EXTENSION
            | ObjectType::OBJECT_FDW
            | ObjectType::OBJECT_PUBLICATION
            | ObjectType::OBJECT_SCHEMA
            | ObjectType::OBJECT_FOREIGN_SERVER => {
                buffer.push(' ');
                NameList(objects).build(buffer)?;
            }
            // drop_type_name_on_any_name
            ObjectType::OBJECT_POLICY | ObjectType::OBJECT_RULE | ObjectType::OBJECT_TRIGGER => {
                // This is a List node with a String array
                let list = iter_only!(objects, Node::List)
                    .next()
                    .ok_or_else(|| SqlError::Missing("List node".into()))?;

                // Weirdly, the last position is the column name - and everything else before that is
                // a fully qualified name.
                if list.items.len() < 2 {
                    return Err(SqlError::Unsupported(format!(
                        "Unsupported format for policy/rule/trigger: {}",
                        list.items.len()
                    )));
                }
                buffer.push(' ');
                let col_id = string_value!(list.items.iter().last().unwrap());
                ColId(col_id).build(buffer)?;
                buffer.push_str(" ON ");
                let remainder = &list.items[0..list.items.len() - 1];
                AnyName(remainder).build(buffer)?;
            }
            ObjectType::OBJECT_CAST => {
                // This is a List node with a TypeName array
                let list = iter_only!(objects, Node::List)
                    .next()
                    .ok_or_else(|| SqlError::Missing("List node".into()))?;
                let types = iter_only!(list.items, Node::TypeName).collect::<Vec<_>>();
                if types.len() != 2 {
                    return Err(SqlError::Unsupported(format!(
                        "Unsupported format for cast: {}",
                        list.items.len()
                    )));
                }
                buffer.push_str(" (");
                types[0].build(buffer)?;
                buffer.push_str(" AS ");
                types[1].build(buffer)?;
                buffer.push(')');
            }
            ObjectType::OBJECT_OPFAMILY | ObjectType::OBJECT_OPCLASS => {
                // This is a List node with a String array
                let list = iter_only!(objects, Node::List)
                    .next()
                    .ok_or_else(|| SqlError::Missing("List node".into()))?;
                if list.items.len() < 2 {
                    return Err(SqlError::Unsupported(format!(
                        "Unsupported format for op family/class: {}",
                        list.items.len()
                    )));
                }

                // The column is in the first position this time, go figure.
                buffer.push(' ');
                AnyName(&list.items[1..]).build(buffer)?;
                buffer.push_str(" USING ");
                ColId(string_value!(list.items[0])).build(buffer)?;
            }
            ObjectType::OBJECT_TRANSFORM => {
                // List with an array of TypeName and String
                let list = iter_only!(objects, Node::List)
                    .next()
                    .ok_or_else(|| SqlError::Missing("List node".into()))?;
                if list.items.len() < 2 {
                    return Err(SqlError::Unsupported(format!(
                        "Unsupported format for transform: {}",
                        list.items.len()
                    )));
                }
                buffer.push_str(" FOR ");
                node!(list.items[0], Node::TypeName).build(buffer)?;
                buffer.push_str(" LANGUAGE ");
                ColId(string_value!(list.items[1])).build(buffer)?;
            }
            ObjectType::OBJECT_LANGUAGE => {
                if objects.is_empty() {
                    return Err(SqlError::Unsupported(
                        "Empty objects for OBJECT_LANGUAGE".into(),
                    ));
                }
                let value = string_value!(objects[0]);
                buffer.push(' ');
                StringLiteral(value).build(buffer)?;
            }
            ObjectType::OBJECT_TYPE | ObjectType::OBJECT_DOMAIN => {
                buffer.push(' ');
                for (index, obj) in iter_only!(objects, Node::TypeName).enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    obj.build(buffer)?;
                }
            }
            ObjectType::OBJECT_AGGREGATE => {
                buffer.push(' ');
                for (index, item) in iter_only!(objects, Node::ObjectWithArgs).enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    AggregateWithArgTypes(item).build(buffer)?;
                }
            }
            ObjectType::OBJECT_FUNCTION
            | ObjectType::OBJECT_PROCEDURE
            | ObjectType::OBJECT_ROUTINE => {
                buffer.push(' ');
                for (index, item) in iter_only!(objects, Node::ObjectWithArgs).enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    FunctionWithArgTypes(item).build(buffer)?;
                }
            }
            ObjectType::OBJECT_OPERATOR => {
                buffer.push(' ');
                for (index, item) in iter_only!(objects, Node::ObjectWithArgs).enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    OperatorWithArgTypes(item).build(buffer)?;
                }
            }
            unsupported => unsupported!(unsupported),
        }

        // A bit unique in that this will append the space
        OptDropBehavior(&self.behavior).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for DropSubscriptionStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let sub_name = must!(self.subname);
        buffer.push_str("DROP SUBSCRIPTION ");
        if self.missing_ok {
            buffer.push_str("IF EXISTS ");
        }
        buffer.push_str(sub_name);
        Ok(())
    }
}

impl SqlBuilder for DropTableSpaceStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let tablespace = must!(self.tablespacename);
        buffer.push_str("DROP TABLESPACE ");
        if self.missing_ok {
            buffer.push_str("IF EXISTS ");
        }
        buffer.push_str(tablespace);
        Ok(())
    }
}

impl SqlBuilder for ExecuteStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.name);
        buffer.push_str("EXECUTE ");
        buffer.push_str(&quote_identifier(name));
        if let Some(ref params) = self.params {
            if !params.is_empty() {
                buffer.push('(');
                ExprList(params).build(buffer)?;
                buffer.push(')');
            }
        }
        Ok(())
    }
}

impl SqlBuilder for ExplainStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("EXPLAIN");
        if let Some(ref options) = self.options {
            if !options.is_empty() {
                buffer.push_str(" (");
                for (index, element) in iter_only!(options, Node::DefElem).enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    GenericDefElemName(must!(element.defname)).build(buffer)?;
                    if let Some(ref arg) = element.arg {
                        match &**arg {
                            Node::Integer { .. } | Node::Float { .. } => {
                                buffer.push(' ');
                                NumericOnly(&**arg).build(buffer)?;
                            }
                            Node::String {
                                value: Some(ref value),
                            } => {
                                buffer.push(' ');
                                BooleanOrString(value).build(buffer)?;
                            }
                            unexpected => {
                                return Err(SqlError::UnexpectedNodeType(unexpected.name()))
                            }
                        }
                    }
                }
                buffer.push(')');
            }
        }

        // "ExplainableStmt" in gram.y
        must!(self.query).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for FuncCall {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let func_name = must!(self.funcname);

        // OVERLAY is a keyword, and only accepts keyword parameter style when used as a keyword and
        // not as a regular function (i.e. pg_catalog.overlay).
        let name = node_vec_to_string_vec(func_name);
        if name.len() == 2 && name[0].eq("pg_catalog") && name[1].eq("overlay") {
            let args = must!(self.args);
            if args.len() == 4 {
                buffer.push_str("OVERLAY(");
                Expr(&args[0]).build(buffer)?;
                buffer.push_str(" PLACING ");
                Expr(&args[1]).build(buffer)?;
                buffer.push_str(" FROM ");
                Expr(&args[2]).build(buffer)?;
                buffer.push_str(" FOR ");
                Expr(&args[3]).build(buffer)?;
                buffer.push(')');
                return Ok(());
            }
        }

        FuncName(func_name).build(buffer)?;

        buffer.push('(');
        if self.agg_distinct {
            buffer.push_str("DISTINCT ");
        }
        if self.agg_star {
            buffer.push('*');
        } else if let Some(ref args) = self.args {
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                }
                if self.func_variadic && index == args.len() - 1 {
                    buffer.push_str("VARIADIC ");
                }
                arg.build(buffer)?;
            }
        }

        if let Some(ref agg_order) = self.agg_order {
            if !self.agg_within_group {
                buffer.push(' ');
                SortClause(agg_order).build(buffer)?;
            }
        }

        buffer.push(')');

        if let Some(ref agg_order) = self.agg_order {
            if self.agg_within_group {
                buffer.push_str(" WITHIN GROUP (");
                SortClause(agg_order).build(buffer)?;
                buffer.push(')');
            }
        }

        if let Some(ref filter) = self.agg_filter {
            buffer.push_str(" FILTER (WHERE ");
            Expr(&**filter).build(buffer)?;
            buffer.push(')');
        }

        if let Some(ref over) = self.over {
            buffer.push_str(" OVER ");
            if let Some(ref name) = over.name {
                buffer.push_str(name);
            } else {
                (**over).build(buffer)?;
            }
        }

        Ok(())
    }
}

impl SqlBuilder for FunctionParameter {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match *self.mode {
            FunctionParameterMode::FUNC_PARAM_IN => {} // Default
            FunctionParameterMode::FUNC_PARAM_OUT => buffer.push_str("OUT"),
            FunctionParameterMode::FUNC_PARAM_INOUT => buffer.push_str("INOUT"),
            FunctionParameterMode::FUNC_PARAM_VARIADIC => buffer.push_str("VARIADIC"),
            FunctionParameterMode::FUNC_PARAM_TABLE => {} // No special annotation
        }
        if let Some(ref name) = self.name {
            if !buffer.ends_with(' ') && !buffer.ends_with('(') {
                buffer.push(' ');
            }
            buffer.push_str(name);
        }
        if let Some(ref arg_type) = self.arg_type {
            if !buffer.ends_with(' ') && !buffer.ends_with('(') {
                buffer.push(' ');
            }
            (**arg_type).build(buffer)?;
        }
        if let Some(ref def_expr) = self.defexpr {
            buffer.push_str(" = ");
            Expr(&**def_expr).build(buffer)?;
        }
        Ok(())
    }
}

impl SqlBuilder for GrantRoleStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let granted_roles = must!(self.granted_roles);
        let grantee_roles = must!(self.grantee_roles);

        if self.is_grant {
            buffer.push_str("GRANT ");
        } else {
            buffer.push_str("REVOKE ");
        }

        for (index, grant) in iter_only!(granted_roles, Node::AccessPriv).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            grant.build(buffer)?;
        }

        if self.is_grant {
            buffer.push_str(" TO ");
        } else {
            buffer.push_str(" FROM ");
        }

        RoleList(grantee_roles).build(buffer)?;
        if self.admin_opt {
            buffer.push_str(" WITH ADMIN OPTION");
        }
        Ok(())
    }
}

impl SqlBuilder for GrantStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if self.is_grant {
            buffer.push_str("GRANT");
        } else {
            buffer.push_str("REVOKE");
            if self.grant_option {
                buffer.push_str(" GRANT OPTION FOR");
            }
        }

        if let Some(ref privileges) = self.privileges {
            if privileges.is_empty() {
                buffer.push_str(" ALL");
            } else {
                buffer.push(' ');
                for (index, privilege) in iter_only!(privileges, Node::AccessPriv).enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    privilege.build(buffer)?;
                }
            }
        } else {
            buffer.push_str(" ALL");
        }

        buffer.push_str(" ON ");

        // "privilege_target" in gram.y
        match *self.targtype {
            GrantTargetType::ACL_TARGET_OBJECT => {
                let objs = must!(self.objects);
                match *self.objtype {
                    ObjectType::OBJECT_TABLE => {
                        QualifiedNameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_SEQUENCE => {
                        buffer.push_str("SEQUENCE ");
                        QualifiedNameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_FDW => {
                        buffer.push_str("FOREIGN DATA WRAPPER ");
                        NameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_FOREIGN_SERVER => {
                        buffer.push_str("FOREIGN SERVER ");
                        NameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_FUNCTION => {
                        buffer.push_str("FUNCTION ");
                        FunctionWithArgTypesList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_PROCEDURE => {
                        buffer.push_str("PROCEDURE ");
                        FunctionWithArgTypesList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_ROUTINE => {
                        buffer.push_str("ROUTINE ");
                        FunctionWithArgTypesList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_DATABASE => {
                        buffer.push_str("DATABASE ");
                        NameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_DOMAIN => {
                        buffer.push_str("DOMAIN ");
                        AnyNameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_LANGUAGE => {
                        buffer.push_str("LANGUAGE ");
                        NameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_LARGEOBJECT => {
                        buffer.push_str("LARGE OBJECT ");
                        NumericOnlyList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_SCHEMA => {
                        buffer.push_str("SCHEMA ");
                        NameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_TABLESPACE => {
                        buffer.push_str("TABLESPACE ");
                        NameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_TYPE => {
                        buffer.push_str("TYPE ");
                        AnyNameList(objs).build(buffer)?;
                    }
                    unexpected => unsupported!(unexpected),
                }
            }
            GrantTargetType::ACL_TARGET_ALL_IN_SCHEMA => {
                let objs = must!(self.objects);
                match *self.objtype {
                    ObjectType::OBJECT_TABLE => {
                        buffer.push_str("ALL TABLES IN SCHEMA ");
                        NameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_SEQUENCE => {
                        buffer.push_str("ALL SEQUENCES IN SCHEMA ");
                        NameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_FUNCTION => {
                        buffer.push_str("ALL FUNCTIONS IN SCHEMA ");
                        NameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_PROCEDURE => {
                        buffer.push_str("ALL PROCEDURES IN SCHEMA ");
                        NameList(objs).build(buffer)?;
                    }
                    ObjectType::OBJECT_ROUTINE => {
                        buffer.push_str("ALL ROUTINES IN SCHEMA ");
                        NameList(objs).build(buffer)?;
                    }
                    unexpected => unsupported!(unexpected),
                }
            }
            GrantTargetType::ACL_TARGET_DEFAULTS => match *self.objtype {
                ObjectType::OBJECT_TABLE => buffer.push_str("TABLES"),
                ObjectType::OBJECT_SEQUENCE => buffer.push_str("SEQUENCES"),
                ObjectType::OBJECT_FUNCTION => buffer.push_str("FUNCTIONS"),
                ObjectType::OBJECT_PROCEDURE => buffer.push_str("PROCEDURES"),
                ObjectType::OBJECT_ROUTINE => buffer.push_str("ROUTINES"),
                unexpected => unsupported!(unexpected),
            },
        }

        if self.is_grant {
            buffer.push_str(" TO ");
        } else {
            buffer.push_str(" FROM ");
        }

        let grantees = must!(self.grantees);
        for (index, grantee) in iter_only!(grantees, Node::RoleSpec).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            grantee.build(buffer)?;
        }

        if self.is_grant && self.grant_option {
            buffer.push_str(" WITH GRANT OPTION");
        }

        OptDropBehavior(&*self.behavior).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for GroupingSet {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match *self.kind {
            GroupingSetKind::GROUPING_SET_EMPTY => buffer.push_str("()"),
            GroupingSetKind::GROUPING_SET_SIMPLE => unsupported!(self.kind),
            GroupingSetKind::GROUPING_SET_ROLLUP => {
                buffer.push_str("ROLLUP (");
                if let Some(ref content) = self.content {
                    ExprList(content).build(buffer)?;
                }
                buffer.push(')');
            }
            GroupingSetKind::GROUPING_SET_CUBE => {
                buffer.push_str("CUBE (");
                if let Some(ref content) = self.content {
                    ExprList(content).build(buffer)?;
                }
                buffer.push(')');
            }
            GroupingSetKind::GROUPING_SET_SETS => {
                buffer.push_str("GROUPING SETS (");
                if let Some(ref content) = self.content {
                    GroupByList(content).build(buffer)?;
                }
                buffer.push(')');
            }
        }
        Ok(())
    }
}

impl SqlBuilder for IndexElem {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if let Some(ref name) = self.name {
            ColId(name).build(buffer)?;
        } else if let Some(ref expr) = self.expr {
            match &**expr {
                Node::FuncCall(func_call) => func_call.build(buffer)?,
                Node::SQLValueFunction(sql_value_function) => sql_value_function.build(buffer)?,
                Node::TypeCast(type_cast) => type_cast.build(buffer)?,
                Node::CoalesceExpr(coalesce_expr) => coalesce_expr.build(buffer)?,
                Node::MinMaxExpr(min_max_expr) => min_max_expr.build(buffer)?,
                Node::XmlExpr(xml_expr) => xml_expr.build(buffer)?,
                Node::XmlSerialize(xml_serialize) => xml_serialize.build(buffer)?,
                other => {
                    buffer.push('(');
                    Expr(other).build(buffer)?;
                    buffer.push(')');
                }
            }
        } else {
            return Err(SqlError::Unsupported("Unsupported IndexElem".into()));
        }

        // Collate
        if let Some(ref collation) = self.collation {
            buffer.push(' ');
            Collate(collation).build(buffer)?;
        }

        if let Some(ref opclass) = self.opclass {
            if !opclass.is_empty() {
                buffer.push(' ');
                AnyName(opclass).build(buffer)?;
                if let Some(ref opclassopts) = self.opclassopts {
                    if !opclassopts.is_empty() {
                        RelOptions(opclassopts).build(buffer)?;
                    }
                }
            }
        }

        match *self.ordering {
            SortByDir::SORTBY_DEFAULT => {}
            SortByDir::SORTBY_ASC => buffer.push_str(" ASC"),
            SortByDir::SORTBY_DESC => buffer.push_str(" DESC"),
            SortByDir::SORTBY_USING => {}
        }

        match *self.nulls_ordering {
            SortByNulls::SORTBY_NULLS_DEFAULT => {}
            SortByNulls::SORTBY_NULLS_FIRST => buffer.push_str(" NULLS FIRST"),
            SortByNulls::SORTBY_NULLS_LAST => buffer.push_str(" NULLS LAST"),
        }
        Ok(())
    }
}

impl SqlBuilder for IndexStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let relation = must!(self.relation);

        buffer.push_str("CREATE");
        if self.unique {
            buffer.push_str(" UNIQUE");
        }
        buffer.push_str(" INDEX");
        if self.concurrent {
            buffer.push_str(" CONCURRENTLY");
        }
        if self.if_not_exists {
            buffer.push_str(" IF NOT EXISTS");
        }
        if let Some(ref name) = self.idxname {
            buffer.push(' ');
            buffer.push_str(name);
        }
        buffer.push_str(" ON ");
        (**relation).build_with_context(buffer, Context::None)?;
        if let Some(ref access_method) = self.access_method {
            if access_method.ne(constants::DEFAULT_INDEX_TYPE) {
                buffer.push_str(" USING ");
                buffer.push_str(&quote_identifier(access_method));
            }
        }
        buffer.push_str(" (");
        if let Some(ref parameters) = self.index_params {
            for (index, parameter) in iter_only!(parameters, Node::IndexElem).enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                }
                parameter.build(buffer)?;
            }
        }
        buffer.push(')');

        if let Some(ref parameters) = self.index_including_params {
            if !parameters.is_empty() {
                buffer.push_str(" INCLUDE (");
                for (index, parameter) in iter_only!(parameters, Node::IndexElem).enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    parameter.build(buffer)?;
                }
                buffer.push(')');
            }
        }

        if let Some(ref options) = self.options {
            buffer.push(' ');
            OptWith(options).build(buffer)?;
        }

        if let Some(ref table_space) = self.table_space {
            buffer.push_str(" TABLESPACE ");
            buffer.push_str(&quote_identifier(table_space));
        }

        if let Some(ref where_clause) = self.where_clause {
            buffer.push(' ');
            WhereClause(&**where_clause).build(buffer)?;
        }
        Ok(())
    }
}

impl SqlBuilder for InferClause {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if let Some(ref elements) = self.index_elems {
            if !elements.is_empty() {
                buffer.push('(');
                for (index, el) in iter_only!(elements, Node::IndexElem).enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    el.build(buffer)?
                }
                buffer.push(')');
            }
        }

        if let Some(ref name) = self.conname {
            if !buffer.ends_with(' ') {
                buffer.push(' ');
            }
            buffer.push_str("ON CONSTRAINT ");
            buffer.push_str(&quote_identifier(name));
        }

        if let Some(ref clause) = self.where_clause {
            if !buffer.ends_with(' ') {
                buffer.push(' ');
            }
            WhereClause(&**clause).build(buffer)?;
        }
        Ok(())
    }
}

impl SqlBuilder for InsertStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if let Some(ref with) = self.with_clause {
            (**with).build(buffer)?;
            buffer.push(' ');
        }

        // Start the insert
        let relation = must!(self.relation);
        buffer.push_str("INSERT INTO ");
        (**relation).build_with_context(buffer, Context::InsertRelation)?;

        // Start the insert columns
        if let Some(ref cols) = self.cols {
            if !cols.is_empty() {
                buffer.push_str(" (");
                InsertColumnList(cols).build(buffer)?;
                buffer.push(')');
            }
        }

        // Overrides as necessary
        match *self.override_ {
            OverridingKind::OVERRIDING_NOT_SET => {} // Do nothing
            OverridingKind::OVERRIDING_USER_VALUE => buffer.push_str(" OVERRIDING USER VALUE"),
            OverridingKind::OVERRIDING_SYSTEM_VALUE => buffer.push_str(" OVERRIDING SYSTEM VALUE"),
        }

        if let Some(ref stmt) = self.select_stmt {
            buffer.push(' ');
            (**stmt).build(buffer)?;
        } else {
            buffer.push_str(" DEFAULT VALUES");
        }

        // on conflict
        if let Some(ref conflict) = self.on_conflict_clause {
            buffer.push(' ');
            (**conflict).build(buffer)?;
        }

        // Returning...
        if let Some(ref returning) = self.returning_list {
            if !returning.is_empty() {
                buffer.push_str(" RETURNING ");
                TargetList(returning).build(buffer)?;
            }
        }

        Ok(())
    }
}

impl SqlBuilder for LoadStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let filename = must!(self.filename);
        buffer.push_str("LOAD ");
        StringLiteral(filename).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for LockStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let relations = must!(self.relations);
        buffer.push_str("LOCK TABLE ");
        RelationExprList(relations).build(buffer)?;
        if self.mode != constants::lock::AccessExclusiveLock {
            buffer.push_str(" IN ");
            match self.mode {
                constants::lock::AccessShareLock => buffer.push_str("ACCESS SHARE"),
                constants::lock::RowShareLock => buffer.push_str("ROW SHARE"),
                constants::lock::RowExclusiveLock => buffer.push_str("ROW EXCLUSIVE"),
                constants::lock::ShareUpdateExclusiveLock => {
                    buffer.push_str("SHARE UPDATE EXCLUSIVE")
                }
                constants::lock::ShareLock => buffer.push_str("SHARE"),
                constants::lock::ShareRowExclusiveLock => buffer.push_str("SHARE ROW EXCLUSIVE"),
                constants::lock::ExclusiveLock => buffer.push_str("EXCLUSIVE"),
                constants::lock::AccessExclusiveLock => buffer.push_str("ACCESS EXCLUSIVE"),
                unsupported => {
                    return Err(SqlError::Unsupported(format!(
                        "Unknown lock type: {}",
                        unsupported
                    )))
                }
            }
            buffer.push_str(" MODE");
        }
        if self.nowait {
            buffer.push_str(" NOWAIT");
        }
        Ok(())
    }
}

impl SqlBuilder for LockingClause {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match *self.strength {
            LockClauseStrength::LCS_NONE => {}
            LockClauseStrength::LCS_FORKEYSHARE => buffer.push_str(" FOR KEY SHARE"),
            LockClauseStrength::LCS_FORSHARE => buffer.push_str(" FOR SHARE"),
            LockClauseStrength::LCS_FORNOKEYUPDATE => buffer.push_str(" FOR NO KEY UPDATE"),
            LockClauseStrength::LCS_FORUPDATE => buffer.push_str(" FOR UPDATE"),
        }

        if let Some(ref rels) = self.locked_rels {
            buffer.push_str(" OF ");
            QualifiedNameList(rels).build(buffer)?;
        }

        match *self.wait_policy {
            LockWaitPolicy::LockWaitBlock => {}
            LockWaitPolicy::LockWaitSkip => buffer.push_str(" SKIP LOCKED"),
            LockWaitPolicy::LockWaitError => buffer.push_str(" NOWAIT"),
        }
        Ok(())
    }
}

impl SqlBuilder for OnConflictClause {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("ON CONFLICT");

        // Infer clause
        if let Some(ref infer) = self.infer {
            buffer.push(' ');
            (**infer).build(buffer)?;
        }

        match *self.action {
            OnConflictAction::ONCONFLICT_NONE => {}
            OnConflictAction::ONCONFLICT_NOTHING => buffer.push_str(" DO NOTHING"),
            OnConflictAction::ONCONFLICT_UPDATE => buffer.push_str(" DO UPDATE"),
        }

        // Target list
        if let Some(ref list) = self.target_list {
            if !list.is_empty() {
                buffer.push_str(" SET ");
                SetClauseList(list).build(buffer)?;
            }
        }

        // Where clause
        if let Some(ref clause) = self.where_clause {
            buffer.push(' ');
            WhereClause(&**clause).build(buffer)?;
        }
        Ok(())
    }
}

impl SqlBuilder for ParamRef {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if self.number == 0 {
            buffer.push('?');
        } else {
            buffer.push_str(&format!("${}", self.number));
        }
        Ok(())
    }
}

impl SqlBuilder for PartitionBoundSpec {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if self.is_default {
            buffer.push_str("DEFAULT");
            return Ok(());
        }

        buffer.push_str("FOR VALUES ");
        match self.strategy {
            constants::PARTITION_STRATEGY_HASH => {
                buffer.push_str(&format!(
                    "WITH (MODULUS {}, REMAINDER {})",
                    self.modulus, self.remainder
                ));
            }
            constants::PARTITION_STRATEGY_LIST => {
                buffer.push_str("IN (");
                if let Some(ref data) = self.listdatums {
                    ExprList(data).build(buffer)?;
                }
                buffer.push(')');
            }
            constants::PARTITION_STRATEGY_RANGE => {
                buffer.push_str("FROM (");
                if let Some(ref data) = self.lowerdatums {
                    ExprList(data).build(buffer)?;
                }
                buffer.push_str(") TO (");
                if let Some(ref data) = self.upperdatums {
                    ExprList(data).build(buffer)?;
                }
                buffer.push(')');
            }
            unexpected => {
                return Err(SqlError::Unsupported(format!(
                    "Partition Strategy: {}",
                    unexpected
                )))
            }
        }

        Ok(())
    }
}

impl SqlBuilder for PartitionCmd {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.name);
        (**name).build_with_context(buffer, Context::None)?;

        if let Some(ref bound) = self.bound {
            buffer.push(' ');
            (**bound).build(buffer)?;
        }
        Ok(())
    }
}

impl SqlBuilder for PartitionElem {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if let Some(ref name) = self.name {
            ColId(name).build(buffer)?;
        } else if let Some(ref expr) = self.expr {
            buffer.push('(');
            Expr(&**expr).build(buffer)?;
            buffer.push(')');
        }

        if let Some(ref collate) = self.collation {
            buffer.push(' ');
            Collate(collate).build(buffer)?;
        }
        if let Some(ref op) = self.opclass {
            buffer.push(' ');
            AnyName(op).build(buffer)?;
        }
        Ok(())
    }
}

impl SqlBuilder for PartitionSpec {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("PARTITION BY ");
        if let Some(ref strategy) = self.strategy {
            buffer.push_str(strategy);
        }
        buffer.push('(');
        if let Some(ref params) = self.part_params {
            for (index, elem) in iter_only!(params, Node::PartitionElem).enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                }
                elem.build(buffer)?;
            }
        }
        buffer.push(')');
        Ok(())
    }
}

impl SqlBuilder for PrepareStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let col_id = must!(self.name);
        let query = must!(self.query);

        buffer.push_str("PREPARE ");
        ColId(col_id).build(buffer)?;
        if let Some(ref argtypes) = self.argtypes {
            if !argtypes.is_empty() {
                buffer.push('(');
                TypeList(argtypes).build(buffer)?;
                buffer.push(')');
            }
        }
        buffer.push_str(" AS ");
        PreparableStmt(query).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for RangeFunction {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if self.lateral {
            if !buffer.is_empty() && !buffer.ends_with(' ') {
                buffer.push(' ');
            }
            buffer.push_str("LATERAL ");
        }
        if self.is_rowsfrom {
            let functions = must!(self.functions);
            if !buffer.is_empty() && !buffer.ends_with(' ') {
                buffer.push(' ');
            }
            buffer.push_str("ROWS FROM (");

            // This is a bizarre structure: List(FuncExprWindowless, List)
            for (index, list) in functions.iter().enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                }

                // Extract the list
                let list = node!(list, Node::List);
                if list.items.len() != 2 {
                    return Err(SqlError::Unsupported("list.items.len() != 2".into()));
                }

                // We could force it by limiting to "func_expr_windowless" in gram.y
                FuncExprWindowless(&list.items[0]).build(buffer)?;
                buffer.push(' ');
                let column_def_list = node!(list.items[1], Node::List);
                if !column_def_list.items.is_empty() {
                    buffer.push_str("AS (");
                    for (index, col) in
                        iter_only!(column_def_list.items, Node::ColumnDef).enumerate()
                    {
                        if index > 0 {
                            buffer.push_str(", ");
                        }
                        col.build(buffer)?;
                    }
                    buffer.push(')');
                }
            }

            buffer.push(')');
        } else if let Some(ref func) = self.functions {
            // Consider error cases when we're not getting what we want
            if let Some(Node::List(list)) = func.first() {
                if let Some(func) = list.items.first() {
                    func.build(buffer)?;
                }
            }
        }
        if self.ordinality {
            if !buffer.is_empty() && !buffer.ends_with(' ') {
                buffer.push(' ');
            }

            buffer.push_str("WITH ORDINALITY ");
        }
        if let Some(ref alias) = self.alias {
            buffer.push(' ');
            (**alias).build(buffer)?;
        }

        if let Some(ref columns) = self.coldeflist {
            if self.alias.is_none() {
                buffer.push_str(" AS ");
            } else {
                buffer.push(' ');
            }
            buffer.push('(');
            let mut iter = columns.iter().peekable();
            while let Some(col) = iter.next() {
                // Consider forcing to column def
                col.build(buffer)?;
                if iter.peek().is_some() {
                    buffer.push_str(", ");
                }
            }
            buffer.push(')');
        }
        Ok(())
    }
}

impl SqlBuilder for RangeSubselect {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        // Extract the subquery since it is mandatory
        let select = must!(self.subquery);

        if self.lateral {
            buffer.push_str("LATERAL ");
        }
        buffer.push('(');
        node!(**select, Node::SelectStmt).build(buffer)?;
        buffer.push(')');

        // Alias if necessary
        if let Some(ref alias) = self.alias {
            buffer.push(' ');
            (**alias).build(buffer)?;
        }

        Ok(())
    }
}

impl SqlBuilder for RangeTableFunc {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if self.lateral {
            buffer.push_str("LATERAL ");
        }
        buffer.push_str("xmltable(");
        if let Some(ref namespaces) = self.namespaces {
            buffer.push_str("xmlnamespaces(");
            XmlNamespaceList(namespaces).build(buffer)?;
            buffer.push_str("), ");
        }

        buffer.push('(');
        let expr = must!(self.rowexpr);
        Expr(&**expr).build(buffer)?;
        buffer.push(')');

        buffer.push_str(" PASSING ");
        let expr = must!(self.docexpr);
        Expr(&**expr).build(buffer)?;

        buffer.push_str(" COLUMNS ");
        let columns = must!(self.columns);
        for (index, col) in iter_only!(columns, Node::RangeTableFuncCol).enumerate() {
            if index > 0 {
                buffer.push_str(", ");
            }
            col.build(buffer)?;
        }

        buffer.push(')');
        if let Some(ref alias) = self.alias {
            buffer.push_str(" AS ");
            (**alias).build(buffer)?;
        }
        Ok(())
    }
}

impl SqlBuilder for RangeTableFuncCol {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let colname = must!(self.colname);
        buffer.push_str(&quote_identifier(colname));

        if self.for_ordinality {
            buffer.push_str(" FOR ORDINALITY");
        } else {
            let type_name = must!(self.type_name);
            buffer.push(' ');
            (**type_name).build(buffer)?;

            if let Some(ref expr) = self.colexpr {
                buffer.push_str(" PATH ");
                Expr(&**expr).build(buffer)?;
            }

            if let Some(ref expr) = self.coldefexpr {
                buffer.push_str(" DEFAULT ");
                Expr(&**expr).build(buffer)?;
            }

            if self.is_not_null {
                buffer.push_str(" NOT NULL");
            }
        }

        Ok(())
    }
}

impl SqlBuilder for RangeTableSample {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let relation = must!(self.relation);
        let relation = node!(**relation, Node::RangeVar);
        let method = must!(self.method);

        // Start building
        relation.build_with_context(buffer, Context::None)?;
        buffer.push_str(" TABLESAMPLE ");
        FuncName(method).build(buffer)?;
        buffer.push('(');
        if let Some(ref args) = self.args {
            ExprList(args).build(buffer)?;
        }
        buffer.push(')');

        if let Some(ref repeatable) = self.repeatable {
            buffer.push_str(" REPEATABLE (");
            Expr(&**repeatable).build(buffer)?;
            buffer.push(')');
        }
        Ok(())
    }
}

impl SqlBuilder for RenameStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("ALTER");
        match *self.rename_type {
            ObjectType::OBJECT_AGGREGATE => buffer.push_str(" AGGREGATE"),
            ObjectType::OBJECT_COLLATION => buffer.push_str(" COLLATION"),
            ObjectType::OBJECT_CONVERSION => buffer.push_str(" CONVERSION"),
            ObjectType::OBJECT_DATABASE => buffer.push_str(" DATABASE"),
            ObjectType::OBJECT_DOMAIN | ObjectType::OBJECT_DOMCONSTRAINT => {
                buffer.push_str(" DOMAIN")
            }
            ObjectType::OBJECT_FDW => buffer.push_str(" FOREIGN DATA WRAPPER"),
            ObjectType::OBJECT_FUNCTION => buffer.push_str(" FUNCTION"),
            ObjectType::OBJECT_ROLE => buffer.push_str(" ROLE"),
            ObjectType::OBJECT_LANGUAGE => buffer.push_str(" LANGUAGE"),
            ObjectType::OBJECT_OPCLASS => buffer.push_str(" OPERATOR CLASS"),
            ObjectType::OBJECT_OPFAMILY => buffer.push_str(" OPERATOR FAMILY"),
            ObjectType::OBJECT_POLICY => buffer.push_str(" POLICY"),
            ObjectType::OBJECT_PROCEDURE => buffer.push_str(" PROCEDURE"),
            ObjectType::OBJECT_PUBLICATION => buffer.push_str(" PUBLICATION"),
            ObjectType::OBJECT_ROUTINE => buffer.push_str(" ROUTINE"),
            ObjectType::OBJECT_SCHEMA => buffer.push_str(" SCHEMA"),
            ObjectType::OBJECT_FOREIGN_SERVER => buffer.push_str(" SERVER"),
            ObjectType::OBJECT_SUBSCRIPTION => buffer.push_str(" SUBSCRIPTION"),
            ObjectType::OBJECT_TABLE | ObjectType::OBJECT_TABCONSTRAINT => {
                buffer.push_str(" TABLE")
            }
            ObjectType::OBJECT_COLUMN => match *self.relation_type {
                ObjectType::OBJECT_TABLE => buffer.push_str(" TABLE"),
                ObjectType::OBJECT_FOREIGN_TABLE => buffer.push_str(" FOREIGN TABLE"),
                ObjectType::OBJECT_VIEW => buffer.push_str(" VIEW"),
                ObjectType::OBJECT_MATVIEW => buffer.push_str(" MATERIALIZED VIEW"),
                ty => unsupported!(ty),
            },
            ObjectType::OBJECT_SEQUENCE => buffer.push_str(" SEQUENCE"),
            ObjectType::OBJECT_VIEW => buffer.push_str(" VIEW"),
            ObjectType::OBJECT_MATVIEW => buffer.push_str(" MATERIALIZED VIEW"),
            ObjectType::OBJECT_INDEX => buffer.push_str(" INDEX"),
            ObjectType::OBJECT_FOREIGN_TABLE => buffer.push_str(" FOREIGN TABLE"),
            ObjectType::OBJECT_RULE => buffer.push_str(" RULE"),
            ObjectType::OBJECT_TRIGGER => buffer.push_str(" TRIGGER"),
            ObjectType::OBJECT_EVENT_TRIGGER => buffer.push_str(" EVENT TRIGGER"),
            ObjectType::OBJECT_TABLESPACE => buffer.push_str(" TABLESPACE"),
            ObjectType::OBJECT_STATISTIC_EXT => buffer.push_str(" STATISTICS"),
            ObjectType::OBJECT_TSPARSER => buffer.push_str(" TEXT SEARCH PARSER"),
            ObjectType::OBJECT_TSDICTIONARY => buffer.push_str(" TEXT SEARCH DICTIONARY"),
            ObjectType::OBJECT_TSTEMPLATE => buffer.push_str(" TEXT SEARCH TEMPLATE"),
            ObjectType::OBJECT_TSCONFIGURATION => buffer.push_str(" TEXT SEARCH CONFIGURATION"),
            ObjectType::OBJECT_TYPE | ObjectType::OBJECT_ATTRIBUTE => buffer.push_str(" TYPE"),
            ty => unsupported!(ty),
        }

        if self.missing_ok {
            buffer.push_str(" IF EXISTS");
        }

        match *self.rename_type {
            ObjectType::OBJECT_AGGREGATE => {
                let object = must!(self.object);
                let object = node!(**object, Node::ObjectWithArgs);
                buffer.push(' ');
                AggregateWithArgTypes(object).build(buffer)?;
                buffer.push_str(" RENAME");
            }
            ObjectType::OBJECT_DOMCONSTRAINT => {
                let object = must!(self.object);
                let list = node!(**object, Node::List);
                buffer.push(' ');
                AnyName(&list.items).build(buffer)?;
                buffer.push_str(" RENAME CONSTRAINT ");
                buffer.push_str(&quote_identifier(must!(self.subname)));
            }
            ObjectType::OBJECT_OPCLASS | ObjectType::OBJECT_OPFAMILY => {
                let object = must!(self.object);
                let list = node!(**object, Node::List);
                // We need to skip the first element in this case
                buffer.push(' ');
                AnyName(&list.items[1..]).build(buffer)?;
                buffer.push_str(" USING ");
                buffer.push_str(&quote_identifier(string_value!(list.items[0])));
                buffer.push_str(" RENAME");
            }
            ObjectType::OBJECT_POLICY => {
                buffer.push(' ');
                buffer.push_str(&quote_identifier(must!(self.subname)));
                buffer.push_str(" ON ");
                must!(self.relation).build_with_context(buffer, Context::None)?;
                buffer.push_str(" RENAME");
            }
            ObjectType::OBJECT_FUNCTION
            | ObjectType::OBJECT_PROCEDURE
            | ObjectType::OBJECT_ROUTINE => {
                let object = must!(self.object);
                let object = node!(**object, Node::ObjectWithArgs);
                buffer.push(' ');
                FunctionWithArgTypes(object).build(buffer)?;
                buffer.push_str(" RENAME");
            }
            ObjectType::OBJECT_SUBSCRIPTION => {
                let object = must!(self.object);
                let col_id = string_value!(**object);
                buffer.push(' ');
                ColId(col_id).build(buffer)?;
                buffer.push_str(" RENAME");
            }
            ObjectType::OBJECT_TABLE
            | ObjectType::OBJECT_SEQUENCE
            | ObjectType::OBJECT_VIEW
            | ObjectType::OBJECT_MATVIEW
            | ObjectType::OBJECT_INDEX
            | ObjectType::OBJECT_FOREIGN_TABLE => {
                buffer.push(' ');
                must!(self.relation).build_with_context(buffer, Context::None)?;
                buffer.push_str(" RENAME");
            }
            ObjectType::OBJECT_COLUMN => {
                buffer.push(' ');
                must!(self.relation).build_with_context(buffer, Context::None)?;
                buffer.push_str(" RENAME COLUMN ");
                buffer.push_str(&quote_identifier(must!(self.subname)));
            }
            ObjectType::OBJECT_TABCONSTRAINT => {
                buffer.push(' ');
                must!(self.relation).build_with_context(buffer, Context::None)?;
                buffer.push_str(" RENAME CONSTRAINT ");
                buffer.push_str(&quote_identifier(must!(self.subname)));
            }
            ObjectType::OBJECT_RULE | ObjectType::OBJECT_TRIGGER => {
                buffer.push(' ');
                buffer.push_str(&quote_identifier(must!(self.subname)));
                buffer.push_str(" ON ");
                must!(self.relation).build_with_context(buffer, Context::None)?;
                buffer.push_str(" RENAME");
            }
            ObjectType::OBJECT_FDW
            | ObjectType::OBJECT_LANGUAGE
            | ObjectType::OBJECT_PUBLICATION
            | ObjectType::OBJECT_FOREIGN_SERVER
            | ObjectType::OBJECT_EVENT_TRIGGER => {
                buffer.push(' ');
                let object = must!(self.object);
                buffer.push_str(&quote_identifier(string_value!(**object)));
                buffer.push_str(" RENAME");
            }
            ObjectType::OBJECT_DATABASE
            | ObjectType::OBJECT_ROLE
            | ObjectType::OBJECT_SCHEMA
            | ObjectType::OBJECT_TABLESPACE => {
                buffer.push(' ');
                buffer.push_str(&quote_identifier(must!(self.subname)));
                buffer.push_str(" RENAME");
            }
            ObjectType::OBJECT_COLLATION
            | ObjectType::OBJECT_CONVERSION
            | ObjectType::OBJECT_DOMAIN
            | ObjectType::OBJECT_STATISTIC_EXT
            | ObjectType::OBJECT_TSPARSER
            | ObjectType::OBJECT_TSDICTIONARY
            | ObjectType::OBJECT_TSTEMPLATE
            | ObjectType::OBJECT_TSCONFIGURATION
            | ObjectType::OBJECT_TYPE => {
                let object = must!(self.object);
                let list = node!(**object, Node::List);
                buffer.push(' ');
                AnyName(&list.items).build(buffer)?;
                buffer.push_str(" RENAME");
            }
            ObjectType::OBJECT_ATTRIBUTE => {
                buffer.push(' ');
                must!(self.relation).build_with_context(buffer, Context::AlterType)?;
                buffer.push_str(" RENAME ATTRIBUTE ");
                buffer.push_str(&quote_identifier(must!(self.subname)));
            }
            other => unsupported!(other),
        }
        buffer.push_str(" TO ");
        buffer.push_str(&quote_identifier(must!(self.newname)));

        OptDropBehavior(&*self.behavior).build(buffer)?;
        Ok(())
    }
}

impl SqlBuilder for ReplicaIdentityStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match self.identity_type {
            constants::REPLICA_IDENTITY_NOTHING => buffer.push_str("NOTHING"),
            constants::REPLICA_IDENTITY_FULL => buffer.push_str("FULL"),
            constants::REPLICA_IDENTITY_DEFAULT => buffer.push_str("DEFAULT"),
            constants::REPLICA_IDENTITY_INDEX => {
                let name = must!(self.name);
                buffer.push_str("USING INDEX ");
                buffer.push_str(&quote_identifier(name));
            }
            _ => {}
        }
        Ok(())
    }
}

impl SqlBuilder for ResTarget {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let val = must!(self.val);
        (**val).build(buffer)?;

        if let Some(ref name) = self.name {
            buffer.push_str(&format!(" AS {}", quote_identifier(name)));
        }

        Ok(())
    }
}

impl SqlBuilder for RoleSpec {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match *self.roletype {
            RoleSpecType::ROLESPEC_CSTRING => {
                buffer.push_str(&quote_identifier(must!(self.rolename)));
            }
            RoleSpecType::ROLESPEC_CURRENT_USER => buffer.push_str("CURRENT_USER"),
            RoleSpecType::ROLESPEC_SESSION_USER => buffer.push_str("SESSION_USER"),
            RoleSpecType::ROLESPEC_PUBLIC => buffer.push_str("public"),
        }
        Ok(())
    }
}

impl SqlBuilder for SelectStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if let Some(ref with) = self.with_clause {
            (**with).build(buffer)?;
        }

        match *self.op {
            SetOperation::SETOP_NONE if self.values_lists.is_some() => {
                let values = must!(self.values_lists);
                buffer.push_str("VALUES ");
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    buffer.push('(');
                    let list = node!(value, Node::List);
                    ExprList(&list.items).build(buffer)?;
                    buffer.push(')');
                }
            }
            SetOperation::SETOP_NONE if self.values_lists.is_none() => {
                if !buffer.is_empty() && !buffer.ends_with('(') && !buffer.ends_with(' ') {
                    buffer.push(' ');
                }
                buffer.push_str("SELECT ");
                if let Some(ref target_list) = self.target_list {
                    // Process the distinct portion (if any)
                    if let Some(ref distinct) = self.distinct_clause {
                        buffer.push_str("DISTINCT ");
                        if !distinct.is_empty() {
                            buffer.push_str("ON (");
                            ExprList(distinct).build(buffer)?;
                            buffer.push_str(") ");
                        }
                    }
                    // Go through the target list
                    for (index, target) in target_list.iter().enumerate() {
                        // We only expect res targets in here
                        let target = node!(target, Node::ResTarget);
                        if index > 0 {
                            buffer.push_str(", ");
                        }
                        target.build(buffer)?;
                    }
                }

                // Generate the into clause
                if let Some(ref into) = self.into_clause {
                    if !buffer.ends_with(' ') {
                        buffer.push(' ');
                    }
                    buffer.push_str("INTO ");
                    if let Some(ref rel) = into.rel {
                        if let Some(persistence) = persistence_from_code(rel.relpersistence) {
                            buffer.push_str(&format!("{} ", persistence));
                        }
                    }

                    // Generate the into
                    (**into).build(buffer)?;
                }

                if let Some(ref from_clause) = self.from_clause {
                    if !buffer.ends_with(' ') {
                        buffer.push(' ');
                    }
                    FromClause(from_clause).build(buffer)?;
                }

                if let Some(ref where_clause) = self.where_clause {
                    buffer.push(' ');
                    WhereClause(&**where_clause).build(buffer)?;
                }

                // Group by
                if let Some(ref group) = self.group_clause {
                    buffer.push_str(" GROUP BY ");
                    GroupByList(group).build(buffer)?;
                }

                // Having
                if let Some(ref having) = self.having_clause {
                    buffer.push_str(" HAVING ");
                    Expr(&**having).build(buffer)?;
                }

                // Window functions
                if let Some(ref window) = self.window_clause {
                    buffer.push_str(" WINDOW ");
                    for (index, node) in window.iter().enumerate() {
                        if index > 0 {
                            buffer.push_str(", ");
                        }
                        let def = node!(node, Node::WindowDef);
                        let name = must!(def.name);
                        buffer.push_str(name);
                        buffer.push_str(" AS ");
                        def.build(buffer)?;
                    }
                }
            }
            SetOperation::SETOP_UNION
            | SetOperation::SETOP_INTERSECT
            | SetOperation::SETOP_EXCEPT => {
                if !buffer.is_empty() && !buffer.ends_with('(') && !buffer.ends_with(' ') {
                    buffer.push(' ');
                }
                let left = must!(self.larg);
                let right = must!(self.rarg);

                fn need_parenthesis(stmt: &SelectStmt) -> bool {
                    if let Some(ref sort) = stmt.sort_clause {
                        if !sort.is_empty() {
                            return true;
                        }
                    }
                    if stmt.limit_offset.is_some()
                        || stmt.limit_count.is_some()
                        || stmt.with_clause.is_some()
                    {
                        return true;
                    }
                    if let Some(ref lock) = stmt.locking_clause {
                        if !lock.is_empty() {
                            return true;
                        }
                    }
                    (*stmt.op).ne(&SetOperation::SETOP_NONE)
                }

                let left_parenthesis = need_parenthesis(left);
                if left_parenthesis {
                    buffer.push('(');
                }
                (**left).build(buffer)?;
                if left_parenthesis {
                    buffer.push(')');
                }

                // Set operations
                match *self.op {
                    SetOperation::SETOP_NONE => {}
                    SetOperation::SETOP_UNION => buffer.push_str(" UNION "),
                    SetOperation::SETOP_INTERSECT => buffer.push_str(" INTERSECT "),
                    SetOperation::SETOP_EXCEPT => buffer.push_str(" EXCEPT "),
                }

                if self.all {
                    buffer.push_str("ALL ");
                }

                let right_parenthesis = need_parenthesis(right);
                if right_parenthesis {
                    buffer.push('(');
                }
                (**right).build(buffer)?;
                if right_parenthesis {
                    buffer.push(')');
                }
            }
            _ => return Err(SqlError::Unsupported("Unsupported select type".into())),
        }

        if let Some(ref sort) = self.sort_clause {
            buffer.push(' ');
            SortClause(sort).build(buffer)?;
        }

        // Limit and limit offset
        if let Some(ref limit) = self.limit_count {
            let ties = match *self.limit_option {
                LimitOption::LIMIT_OPTION_DEFAULT => false, // Ignore
                LimitOption::LIMIT_OPTION_COUNT => {
                    if !buffer.ends_with(' ') {
                        buffer.push(' ');
                    }
                    buffer.push_str("LIMIT ");
                    false
                }
                LimitOption::LIMIT_OPTION_WITH_TIES => {
                    if !buffer.ends_with(' ') {
                        buffer.push(' ');
                    }
                    buffer.push_str("FETCH FIRST ");
                    true
                }
            };

            let all = if let Node::A_Const(ref a_const) = **limit {
                matches!(a_const.val.0, Node::Null {})
            } else {
                false
            };
            if all {
                buffer.push_str("ALL");
            } else {
                // We could break this into a separate function (e.g. as per "c_expr" in gram.y)
                if let Node::A_Expr(ref expr) = **limit {
                    buffer.push('(');
                    expr.build_with_context(buffer, Context::None)?;
                    buffer.push(')');
                } else {
                    limit.build(buffer)?;
                }
            }

            if ties {
                buffer.push_str(" ROWS WITH TIES")
            }
        }

        if let Some(ref offset) = self.limit_offset {
            if !buffer.ends_with(' ') {
                buffer.push(' ');
            }
            buffer.push_str("OFFSET ");
            Expr(&**offset).build(buffer)?;
        }

        if let Some(ref locking) = self.locking_clause {
            for (index, clause) in iter_only!(locking, Node::LockingClause).enumerate() {
                if index > 0 {
                    buffer.push(' ');
                }
                clause.build(buffer)?;
            }
        }

        Ok(())
    }
}

impl SqlBuilder for SortBy {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let node = must!(self.node);
        Expr(&**node).build(buffer)?;

        // Sort order
        match *self.sortby_dir {
            SortByDir::SORTBY_DEFAULT => {}
            SortByDir::SORTBY_ASC => buffer.push_str(" ASC"),
            SortByDir::SORTBY_DESC => buffer.push_str(" DESC"),
            SortByDir::SORTBY_USING => {
                buffer.push_str(" USING ");
                let use_op = must!(self.use_op);
                QualifiedOperator(use_op).build(buffer)?;
            }
        }

        // Null ordering
        match *self.sortby_nulls {
            SortByNulls::SORTBY_NULLS_DEFAULT => {}
            SortByNulls::SORTBY_NULLS_FIRST => buffer.push_str(" NULLS FIRST"),
            SortByNulls::SORTBY_NULLS_LAST => buffer.push_str(" NULLS LAST"),
        }

        Ok(())
    }
}

impl SqlBuilder for TableLikeClause {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let relation = must!(self.relation);

        buffer.push_str("LIKE ");
        (**relation).build_with_context(buffer, Context::None)?;
        if self.options == TableLikeOption::CREATE_TABLE_LIKE_ALL as u32 {
            buffer.push_str(" INCLUDING ALL");
        } else {
            if self.options & TableLikeOption::CREATE_TABLE_LIKE_COMMENTS as u32 > 0 {
                buffer.push_str(" INCLUDING COMMENTS");
            }
            if self.options & TableLikeOption::CREATE_TABLE_LIKE_CONSTRAINTS as u32 > 0 {
                buffer.push_str(" INCLUDING CONSTRAINTS");
            }
            if self.options & TableLikeOption::CREATE_TABLE_LIKE_DEFAULTS as u32 > 0 {
                buffer.push_str(" INCLUDING DEFAULTS");
            }
            if self.options & TableLikeOption::CREATE_TABLE_LIKE_IDENTITY as u32 > 0 {
                buffer.push_str(" INCLUDING IDENTITY");
            }
            if self.options & TableLikeOption::CREATE_TABLE_LIKE_GENERATED as u32 > 0 {
                buffer.push_str(" INCLUDING GENERATED");
            }
            if self.options & TableLikeOption::CREATE_TABLE_LIKE_INDEXES as u32 > 0 {
                buffer.push_str(" INCLUDING INDEXES");
            }
            if self.options & TableLikeOption::CREATE_TABLE_LIKE_STATISTICS as u32 > 0 {
                buffer.push_str(" INCLUDING STATISTICS");
            }
            if self.options & TableLikeOption::CREATE_TABLE_LIKE_STORAGE as u32 > 0 {
                buffer.push_str(" INCLUDING STORAGE");
            }
        }

        Ok(())
    }
}

impl SqlBuilder for TransactionStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match *self.kind {
            TransactionStmtKind::TRANS_STMT_BEGIN => {
                buffer.push_str("BEGIN");
                if let Some(ref options) = self.options {
                    buffer.push(' ');
                    TransactionModeList(options).build(buffer)?;
                }
            }
            TransactionStmtKind::TRANS_STMT_START => {
                buffer.push_str("START TRANSACTION");
                if let Some(ref options) = self.options {
                    buffer.push(' ');
                    TransactionModeList(options).build(buffer)?;
                }
            }
            TransactionStmtKind::TRANS_STMT_COMMIT => {
                buffer.push_str("COMMIT");
                if self.chain {
                    buffer.push_str(" AND CHAIN")
                }
            }
            TransactionStmtKind::TRANS_STMT_ROLLBACK => {
                buffer.push_str("ROLLBACK");
                if self.chain {
                    buffer.push_str(" AND CHAIN")
                }
            }
            TransactionStmtKind::TRANS_STMT_SAVEPOINT => {
                buffer.push_str("SAVEPOINT ");
                buffer.push_str(&quote_identifier(must!(self.savepoint_name)));
            }
            TransactionStmtKind::TRANS_STMT_RELEASE => {
                buffer.push_str("RELEASE ");
                buffer.push_str(&quote_identifier(must!(self.savepoint_name)));
            }
            TransactionStmtKind::TRANS_STMT_ROLLBACK_TO => {
                buffer.push_str("ROLLBACK TO SAVEPOINT ");
                buffer.push_str(&quote_identifier(must!(self.savepoint_name)));
            }
            TransactionStmtKind::TRANS_STMT_PREPARE => {
                buffer.push_str("PREPARE TRANSACTION ");
                StringLiteral(must!(self.gid)).build(buffer)?;
            }
            TransactionStmtKind::TRANS_STMT_COMMIT_PREPARED => {
                buffer.push_str("COMMIT PREPARED ");
                StringLiteral(must!(self.gid)).build(buffer)?;
            }
            TransactionStmtKind::TRANS_STMT_ROLLBACK_PREPARED => {
                buffer.push_str("ROLLBACK PREPARED ");
                StringLiteral(must!(self.gid)).build(buffer)?;
            }
        }
        Ok(())
    }
}

impl SqlBuilder for TriggerTransition {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.name);
        if self.is_new {
            buffer.push_str("NEW ");
        } else {
            buffer.push_str("OLD ");
        }
        if self.is_table {
            buffer.push_str("TABLE ");
        } else {
            buffer.push_str("ROW ");
        }
        buffer.push_str(&quote_identifier(name));
        Ok(())
    }
}

impl SqlBuilder for TypeCast {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let arg = must!(self.arg);
        let type_name = must!(self.type_name);

        // Figure out the right type
        let mut parenthesis = false;
        match &**arg {
            Node::A_Expr(_expr) => {
                buffer.push_str("CAST(");
                Expr(arg).build(buffer)?;
                buffer.push_str(" AS ");
                type_name.build(buffer)?;
                buffer.push(')');
                return Ok(());
            }
            Node::A_Const(a_const) => {
                let names = must!(type_name.names);
                let names = node_vec_to_string_vec(names);
                if names.len() == 2 && names[0].eq("pg_catalog") {
                    let ty = names[1];
                    if ty.eq("bpchar") && type_name.typmods.is_none() {
                        buffer.push_str("char ");
                        a_const.build(buffer)?;
                        return Ok(());
                    }
                    if ty.eq("bool") {
                        if let Node::String {
                            value: Some(ref value),
                        } = a_const.val.0
                        {
                            match &value[..] {
                                "t" => buffer.push_str("true"),
                                "f" => buffer.push_str("false"),
                                _ => {}
                            }
                            return Ok(());
                        }
                    }
                }

                // This ensures negative values have wrapping parens
                match a_const.val.0 {
                    Node::Float { .. } => parenthesis = true,
                    Node::Integer { value } if value < 0 => parenthesis = true,
                    _ => {}
                }
            }
            _ => {}
        }

        // Finally do the cast
        if parenthesis {
            buffer.push('(');
        }
        Expr(&**arg).build(buffer)?;
        if parenthesis {
            buffer.push(')');
        }
        buffer.push_str("::");
        (**type_name).build(buffer)?;

        Ok(())
    }
}

impl SqlBuilder for TypeName {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name_nodes = must!(self.names);
        let names = node_vec_to_string_vec(name_nodes);

        // Track whether we want to skip type modifications
        let mut skip_typmods = false;
        let empty = Vec::new();
        let typmods = self.typmods.as_ref().unwrap_or(&empty);

        if self.setof {
            buffer.push_str("SETOF ");
        }

        // Built in types first
        if names.len() == 2 && names[0].eq("pg_catalog") {
            let other = names[1];
            match &other[..] {
                "bpchar" => buffer.push_str("char"),
                "varchar" => buffer.push_str("varchar"),
                "numeric" => buffer.push_str("numeric"),
                "bool" => buffer.push_str("boolean"),
                "int2" => buffer.push_str("smallint"),
                "int4" => buffer.push_str("int"),
                "int8" => buffer.push_str("bigint"),
                "real" | "float4" => buffer.push_str("real"),
                "float8" => buffer.push_str("double precision"),
                "time" => buffer.push_str("time"),
                "timetz" => {
                    buffer.push_str("time ");
                    if !typmods.is_empty() {
                        buffer.push('(');
                        for (index, typmod) in typmods.iter().enumerate() {
                            if index > 0 {
                                buffer.push_str(", ");
                            }
                            SignedIConst(typmod).build(buffer)?;
                        }
                        buffer.push(')');
                    }
                    buffer.push_str("with time zone");
                    skip_typmods = true;
                }
                "timestamp" => buffer.push_str("timestamp"),
                "timestamptz" => {
                    buffer.push_str("timestamp ");
                    if !typmods.is_empty() {
                        buffer.push('(');
                        for (index, typmod) in typmods.iter().enumerate() {
                            if index > 0 {
                                buffer.push_str(", ");
                            }
                            SignedIConst(typmod).build(buffer)?;
                        }
                        buffer.push(')');
                    }
                    buffer.push_str("with time zone");
                    skip_typmods = true;
                }
                "interval" if typmods.is_empty() => buffer.push_str("interval"),
                "interval" if !typmods.is_empty() => {
                    let a_const = node!(typmods[0], Node::A_Const);
                    let fields = int_value!((*a_const.val).0);

                    buffer.push_str("interval");

                    // This logic is based on intervaltypmodout in timestamp.c
                    match fields {
                        constants::interval::YEAR => buffer.push_str(" year"),
                        constants::interval::MONTH => buffer.push_str(" month"),
                        constants::interval::DAY => buffer.push_str(" day"),
                        constants::interval::HOUR => buffer.push_str(" hour"),
                        constants::interval::MINUTE => buffer.push_str(" minute"),
                        constants::interval::SECOND => buffer.push_str(" second"),
                        constants::interval::YEAR_MONTH => buffer.push_str(" year to month"),
                        constants::interval::DAY_HOUR => buffer.push_str(" day to hour"),
                        constants::interval::DAY_HOUR_MINUTE => buffer.push_str(" day to minute"),
                        constants::interval::DAY_HOUR_MINUTE_SECOND => {
                            buffer.push_str(" day to second")
                        }
                        constants::interval::HOUR_MINUTE => buffer.push_str(" hour to minute"),
                        constants::interval::HOUR_MINUTE_SECOND => {
                            buffer.push_str(" hour to second")
                        }
                        constants::interval::MINUTE_SECOND => buffer.push_str(" minute to second"),
                        constants::interval::FULL_RANGE => {} // nothin
                        unexpected => {
                            return Err(SqlError::Unsupported(format!(
                                "Unexpected interval: {}",
                                unexpected
                            )))
                        }
                    }

                    if let Some(ref mods) = self.typmods {
                        if mods.len() == 2 {
                            let a_const = node!(mods[1], Node::A_Const);
                            let value = int_value!((*(*a_const).val).0);
                            if value != constants::interval::FULL_PRECISION {
                                buffer.push_str(&format!("({})", value))
                            } else {
                                return Err(SqlError::Unsupported(
                                    "FULL_PRECISION for typmods".into(),
                                ));
                            }
                        }
                    }

                    skip_typmods = true;
                }
                name => buffer.push_str(&format!("pg_catalog.{}", name)),
            }
        } else {
            AnyName(name_nodes).build(buffer)?;
        }

        // Do the type mods if need be
        if !typmods.is_empty() && !skip_typmods {
            buffer.push('(');
            for (index, typ) in typmods.iter().enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                }
                match typ {
                    Node::A_Const(a_const) => a_const.build(buffer)?,
                    Node::ParamRef(param_ref) => param_ref.build(buffer)?,
                    Node::ColumnRef(column_ref) => column_ref.build(buffer)?,
                    ty => return Err(SqlError::UnexpectedNodeType(ty.name())),
                }
            }
            buffer.push(')');
        }

        // Array bounds
        if let Some(ref bounds) = self.array_bounds {
            for bound in bounds {
                buffer.push('[');
                match &bound {
                    Node::Integer { value } if *value >= 0 => {
                        buffer.push_str(&(*value).to_string())
                    }
                    _ => {} // Ignore
                }
                buffer.push(']');
            }
        }

        if self.pct_type {
            buffer.push_str("%type");
        }

        Ok(())
    }
}

impl SqlBuilder for UpdateStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let relation = must!(self.relation);

        if let Some(ref with) = self.with_clause {
            (**with).build(buffer)?;
            buffer.push(' ');
        }

        // Start the update
        buffer.push_str("UPDATE ");
        (**relation).build_with_context(buffer, Context::None)?;

        // Start the target list
        if let Some(ref list) = self.target_list {
            if !list.is_empty() {
                buffer.push_str(" SET ");
                SetClauseList(list).build(buffer)?;
            }
        }

        if let Some(ref from) = self.from_clause {
            buffer.push(' ');
            FromClause(from).build(buffer)?;
        }
        if let Some(ref clause) = self.where_clause {
            buffer.push(' ');
            WhereClause(&**clause).build(buffer)?;
        }

        if let Some(ref returning) = self.returning_list {
            if !returning.is_empty() {
                buffer.push_str(" RETURNING ");
                TargetList(returning).build(buffer)?;
            }
        }

        Ok(())
    }
}

impl SqlBuilder for VacuumStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if self.is_vacuumcmd {
            buffer.push_str("VACUUM");
        } else {
            buffer.push_str("ANALYZE");
        }

        if let Some(ref options) = self.options {
            if !options.is_empty() {
                buffer.push_str(" (");
                for (index, option) in iter_only!(options, Node::DefElem).enumerate() {
                    if index > 0 {
                        buffer.push_str(", ");
                    }
                    GenericDefElemName(must!(option.defname)).build(buffer)?;
                    if let Some(ref arg) = option.arg {
                        buffer.push(' ');
                        match &**arg {
                            Node::Integer { .. } | Node::Float { .. } => {
                                NumericOnly(&**arg).build(buffer)?
                            }
                            Node::String {
                                value: Some(ref value),
                            } => BooleanOrString(value).build(buffer)?,
                            unexpected => {
                                return Err(SqlError::UnexpectedNodeType(unexpected.name()))
                            }
                        }
                    }
                }
                buffer.push(')');
            }
        }

        if let Some(ref rels) = self.rels {
            for (index, rel) in iter_only!(rels, Node::VacuumRelation).enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                } else {
                    buffer.push(' ');
                }
                must!(rel.relation).build_with_context(buffer, Context::None)?;
                if let Some(ref cols) = rel.va_cols {
                    if !cols.is_empty() {
                        buffer.push('(');
                        for (sub_index, col) in cols.iter().enumerate() {
                            if sub_index > 0 {
                                buffer.push_str(", ");
                            }
                            buffer.push_str(&quote_identifier(string_value!(col)));
                        }
                        buffer.push(')');
                    }
                }
            }
        }
        Ok(())
    }
}

impl SqlBuilder for VariableSetStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match *self.kind {
            // SET var = value
            VariableSetKind::VAR_SET_VALUE => {
                buffer.push_str("SET ");
                if self.is_local {
                    buffer.push_str("LOCAL ");
                }
                if let Some(ref name) = self.name {
                    VarName(name).build(buffer)?;
                }
                buffer.push_str(" TO ");
                if let Some(ref args) = self.args {
                    VarList(args).build(buffer)?;
                }
            }
            VariableSetKind::VAR_SET_DEFAULT => {
                buffer.push_str("SET ");
                if self.is_local {
                    buffer.push_str("LOCAL ");
                }
                if let Some(ref name) = self.name {
                    VarName(name).build(buffer)?;
                }
                buffer.push_str(" TO DEFAULT");
            }
            VariableSetKind::VAR_SET_CURRENT => {
                buffer.push_str("SET ");
                if self.is_local {
                    buffer.push_str("LOCAL ");
                }
                if let Some(ref name) = self.name {
                    VarName(name).build(buffer)?;
                }
                buffer.push_str(" FROM CURRENT");
            }
            VariableSetKind::VAR_SET_MULTI => {
                let name = must!(self.name);
                buffer.push_str("SET ");
                if self.is_local {
                    buffer.push_str("LOCAL ");
                }
                match &name[..] {
                    "TRANSACTION" => {
                        buffer.push_str("TRANSACTION ");
                        let args = must!(self.args);
                        TransactionModeList(args).build(buffer)?;
                    }
                    "SESSION CHARACTERISTICS" => {
                        buffer.push_str("SESSION CHARACTERISTICS AS TRANSACTION ");
                        let args = must!(self.args);
                        TransactionModeList(args).build(buffer)?;
                    }
                    "TRANSACTION SNAPSHOT" => {
                        buffer.push_str("TRANSACTION SNAPSHOT ");
                        let args = must!(self.args);
                        if args.is_empty() {
                            return Err(SqlError::Missing("args".into()));
                        }
                        let arg = node!(args[0], Node::A_Const);
                        StringLiteral(string_value!((*arg.val).0)).build(buffer)?;
                    }
                    unsupported => {
                        return Err(SqlError::Unsupported(format!(
                            "Unsupported set type: {}",
                            unsupported
                        )))
                    }
                }
            }
            VariableSetKind::VAR_RESET => {
                buffer.push_str("RESET ");
                if let Some(ref name) = self.name {
                    VarName(name).build(buffer)?;
                }
            }
            VariableSetKind::VAR_RESET_ALL => buffer.push_str("RESET ALL"),
        }
        Ok(())
    }
}

impl SqlBuilder for ViewStmt {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let view = must!(self.view);
        buffer.push_str("CREATE ");
        if self.replace {
            buffer.push_str("OR REPLACE ");
        }
        if let Some(persistence) = persistence_from_code(view.relpersistence) {
            buffer.push_str(persistence);
            buffer.push(' ');
        }
        buffer.push_str("VIEW ");
        (**view).build_with_context(buffer, Context::None)?;

        // Aliases
        if let Some(ref aliases) = self.aliases {
            if !aliases.is_empty() {
                buffer.push_str(" (");
                ColumnList(aliases).build(buffer)?;
                buffer.push(')');
            }
        }

        if let Some(ref options) = self.options {
            buffer.push(' ');
            OptWith(options).build(buffer)?;
        }

        if !buffer.ends_with(' ') {
            buffer.push(' ');
        }
        buffer.push_str("AS ");
        if let Some(ref query) = self.query {
            node!(**query, Node::SelectStmt).build(buffer)?;
        }

        // Check options
        match *self.with_check_option {
            ViewCheckOption::NO_CHECK_OPTION => {}
            ViewCheckOption::LOCAL_CHECK_OPTION => buffer.push_str(" WITH LOCAL CHECK OPTION"),
            ViewCheckOption::CASCADED_CHECK_OPTION => buffer.push_str(" WITH CHECK OPTION"),
        }
        Ok(())
    }
}

impl SqlBuilder for WindowDef {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        // The calling node is responsible for outputting the name
        buffer.push('(');

        if let Some(ref ref_name) = self.refname {
            buffer.push_str(&quote_identifier(ref_name));
        }

        if let Some(ref partition) = self.partition_clause {
            if !partition.is_empty() {
                if !buffer.ends_with('(') && !buffer.ends_with(' ') {
                    buffer.push(' ');
                }
                buffer.push_str("PARTITION BY ");
                ExprList(partition).build(buffer)?;
            }
        }

        if let Some(ref sort) = self.order_clause {
            if !buffer.ends_with('(') && !buffer.ends_with(' ') {
                buffer.push(' ');
            }
            SortClause(sort).build(buffer)?;
        }

        if self.frame_options & constants::FRAMEOPTION_NONDEFAULT > 0 {
            if self.frame_options & constants::FRAMEOPTION_RANGE > 0 {
                buffer.push_str(" RANGE");
            } else if self.frame_options & constants::FRAMEOPTION_ROWS > 0 {
                buffer.push_str(" ROWS");
            } else if self.frame_options & constants::FRAMEOPTION_GROUPS > 0 {
                buffer.push_str(" GROUPS");
            }

            if self.frame_options & constants::FRAMEOPTION_BETWEEN > 0 {
                buffer.push_str(" BETWEEN");
            }

            // frame_start
            if self.frame_options & constants::FRAMEOPTION_START_UNBOUNDED_PRECEDING > 0 {
                buffer.push_str(" UNBOUNDED PRECEDING");
            } else if self.frame_options & constants::FRAMEOPTION_START_UNBOUNDED_FOLLOWING > 0 {
                return Err(SqlError::Unsupported(
                    "FRAMEOPTION_START_UNBOUNDED_FOLLOWING disallowed".into(),
                ));
            } else if self.frame_options & constants::FRAMEOPTION_START_CURRENT_ROW > 0 {
                buffer.push_str(" CURRENT ROW");
            } else if self.frame_options & constants::FRAMEOPTION_START_OFFSET_PRECEDING > 0 {
                let start_offset = must!(self.start_offset);
                buffer.push(' ');
                Expr(&**start_offset).build(buffer)?;
                buffer.push_str(" PRECEDING");
            } else if self.frame_options & constants::FRAMEOPTION_START_OFFSET_FOLLOWING > 0 {
                let start_offset = must!(self.start_offset);
                buffer.push(' ');
                Expr(&**start_offset).build(buffer)?;
                buffer.push_str(" FOLLOWING");
            }

            if self.frame_options & constants::FRAMEOPTION_BETWEEN > 0 {
                buffer.push_str(" AND");
                // frame_end
                if self.frame_options & constants::FRAMEOPTION_END_UNBOUNDED_PRECEDING > 0 {
                    return Err(SqlError::Unsupported(
                        "FRAMEOPTION_END_UNBOUNDED_PRECEDING disallowed".into(),
                    ));
                } else if self.frame_options & constants::FRAMEOPTION_END_UNBOUNDED_FOLLOWING > 0 {
                    buffer.push_str(" UNBOUNDED FOLLOWING");
                } else if self.frame_options & constants::FRAMEOPTION_END_CURRENT_ROW > 0 {
                    buffer.push_str(" CURRENT ROW");
                } else if self.frame_options & constants::FRAMEOPTION_END_OFFSET_PRECEDING > 0 {
                    let end_offset = must!(self.end_offset);
                    buffer.push(' ');
                    Expr(&**end_offset).build(buffer)?;
                    buffer.push_str(" PRECEDING");
                } else if self.frame_options & constants::FRAMEOPTION_END_OFFSET_FOLLOWING > 0 {
                    let end_offset = must!(self.end_offset);
                    buffer.push(' ');
                    Expr(&**end_offset).build(buffer)?;
                    buffer.push_str(" FOLLOWING");
                }
            }

            if self.frame_options & constants::FRAMEOPTION_EXCLUDE_CURRENT_ROW > 0 {
                buffer.push_str(" EXCLUDE CURRENT ROW");
            } else if self.frame_options & constants::FRAMEOPTION_EXCLUDE_GROUP > 0 {
                buffer.push_str(" EXCLUDE GROUP");
            } else if self.frame_options & constants::FRAMEOPTION_EXCLUDE_TIES > 0 {
                buffer.push_str(" EXCLUDE TIES");
            }
        }

        buffer.push(')');
        Ok(())
    }
}

impl SqlBuilder for WithClause {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("WITH ");
        if self.recursive {
            buffer.push_str("RECURSIVE ");
        }
        if let Some(ref ctes) = self.ctes {
            for (index, node) in ctes.iter().enumerate() {
                if index > 0 {
                    buffer.push_str(", ");
                }

                let cte = node!(node, Node::CommonTableExpr);
                cte.build(buffer)?;
            }
        }
        Ok(())
    }
}

impl SqlBuilder for XmlSerialize {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("xmlserialize(");
        match *self.xmloption {
            XmlOptionType::XMLOPTION_DOCUMENT => buffer.push_str("document "),
            XmlOptionType::XMLOPTION_CONTENT => buffer.push_str("content "),
        }
        if let Some(ref expr) = self.expr {
            Expr(&**expr).build(buffer)?;
        }
        buffer.push_str(" AS ");
        if let Some(ref type_name) = self.type_name {
            (**type_name).build(buffer)?;
        }
        buffer.push(')');
        Ok(())
    }
}

impl SqlBuilder for Alias {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.aliasname);
        buffer.push_str(&quote_identifier(name));

        // Also process the column names
        if let Some(ref columns) = self.colnames {
            if !columns.is_empty() {
                buffer.push('(');
                NameList(columns).build(buffer)?;
                buffer.push(')');
            }
        }
        Ok(())
    }
}

// Expressions
impl SqlBuilder for BoolExpr {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let args = must!(self.args);
        match *self.boolop {
            BoolExprType::AND_EXPR | BoolExprType::OR_EXPR => {
                let mut iter = args.iter().peekable();
                while let Some(arg) = iter.next() {
                    let parenthesis = if let Node::BoolExpr(be) = &arg {
                        (*be.boolop).eq(&BoolExprType::AND_EXPR)
                            || (*be.boolop).eq(&BoolExprType::OR_EXPR)
                    } else {
                        false
                    };
                    if parenthesis {
                        buffer.push('(');
                    }
                    Expr(arg).build(buffer)?;
                    if parenthesis {
                        buffer.push(')');
                    }
                    if iter.peek().is_some() {
                        buffer.push_str(&format!(" {} ", *self.boolop));
                    }
                }
            }
            BoolExprType::NOT_EXPR => {
                // Must be only one arg
                if args.len() != 1 {
                    return Err(SqlError::Unsupported("args.len() != 1".into()));
                }
                let arg = &args[0];
                let parenthesis = if let Node::BoolExpr(b) = arg {
                    (*b.boolop).eq(&BoolExprType::AND_EXPR)
                        || (*b.boolop).eq(&BoolExprType::OR_EXPR)
                } else {
                    false
                };

                buffer.push_str("NOT ");
                if parenthesis {
                    buffer.push('(');
                }
                Expr(arg).build(buffer)?;
                if parenthesis {
                    buffer.push('(');
                }
            }
        }
        Ok(())
    }
}

impl SqlBuilder for BooleanTest {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let arg = must!(self.arg);
        Expr(&**arg).build(buffer)?;
        match *self.booltesttype {
            BoolTestType::IS_TRUE => buffer.push_str(" IS TRUE"),
            BoolTestType::IS_NOT_TRUE => buffer.push_str(" IS NOT TRUE"),
            BoolTestType::IS_FALSE => buffer.push_str(" IS FALSE"),
            BoolTestType::IS_NOT_FALSE => buffer.push_str(" IS NOT FALSE"),
            BoolTestType::IS_UNKNOWN => buffer.push_str(" IS UNKNOWN"),
            BoolTestType::IS_NOT_UNKNOWN => buffer.push_str(" IS NOT UNKNOWN"),
        }
        Ok(())
    }
}

impl SqlBuilder for CaseExpr {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("CASE ");

        // Do the case expr
        if let Some(ref arg) = self.arg {
            Expr(&**arg).build(buffer)?;
            buffer.push(' ');
        }

        if let Some(ref args) = self.args {
            for arg in args {
                node!(arg, Node::CaseWhen).build(buffer)?;
                buffer.push(' ');
            }
        }

        // else clause
        if let Some(ref else_clause) = self.defresult {
            buffer.push_str("ELSE ");
            Expr(&**else_clause).build(buffer)?;
            buffer.push(' ');
        }

        buffer.push_str("END");
        Ok(())
    }
}

impl SqlBuilder for CaseWhen {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let expr = must!(self.expr);
        let result = must!(self.result);
        buffer.push_str("WHEN ");
        Expr(&**expr).build(buffer)?;
        buffer.push_str(" THEN ");
        Expr(&**result).build(buffer)
    }
}

impl SqlBuilder for CoalesceExpr {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let args = must!(self.args);
        buffer.push_str("COALESCE(");
        ExprList(args).build(buffer)?;
        buffer.push(')');
        Ok(())
    }
}

impl SqlBuilder for CurrentOfExpr {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let name = must!(self.cursor_name);
        buffer.push_str("CURRENT OF ");
        buffer.push_str(&quote_identifier(name));
        Ok(())
    }
}

impl SqlBuilder for GroupingFunc {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("GROUPING(");
        if let Some(ref args) = self.args {
            ExprList(args).build(buffer)?;
        }
        buffer.push(')');
        Ok(())
    }
}

impl SqlBuilder for IntoClause {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let rel = must!(self.rel);

        // Start with the target relation name
        (**rel).build_with_context(buffer, Context::None)?;

        if let Some(ref column_names) = self.col_names {
            if !column_names.is_empty() {
                buffer.push('(');
                ColumnList(column_names).build(buffer)?;
                buffer.push(')');
            }
        }

        // Access method
        if let Some(ref method) = self.access_method {
            buffer.push_str(" USING ");
            buffer.push_str(&quote_identifier(method));
        }

        // With
        if let Some(ref options) = self.options {
            buffer.push(' ');
            OptWith(options).build(buffer)?;
        }

        // ON COMMIT
        match *self.on_commit {
            OnCommitAction::ONCOMMIT_NOOP => {}
            OnCommitAction::ONCOMMIT_PRESERVE_ROWS => buffer.push_str(" ON COMMIT PRESERVE ROWS"),
            OnCommitAction::ONCOMMIT_DELETE_ROWS => buffer.push_str(" ON COMMIT DELETE ROWS"),
            OnCommitAction::ONCOMMIT_DROP => buffer.push_str(" ON COMMIT DROP"),
        }

        if let Some(ref tablespace) = self.table_space_name {
            buffer.push_str(" TABLESPACE ");
            buffer.push_str(&quote_identifier(tablespace));
        }
        Ok(())
    }
}

impl SqlBuilder for JoinExpr {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        if self.alias.is_some() {
            buffer.push('(');
        }

        let table_ref = must!(self.larg);
        let join_ref = must!(self.rarg);

        // Do the table reference.
        TableRef(&**table_ref).build(buffer)?;
        buffer.push(' ');

        // Is it a natural join?
        if self.is_natural {
            buffer.push_str("NATURAL ");
        }

        // What kind of join type
        let empty = Vec::new();
        let using = self.using_clause.as_ref().unwrap_or(&empty);
        match *self.jointype {
            JoinType::JOIN_INNER => {
                // Since INNER JOIN is the default we need to determine whether it is a CROSS JOIN
                if !self.is_natural && self.quals.is_none() && using.is_empty() {
                    buffer.push_str("CROSS ");
                } else {
                    // I prefer these being explicit
                    buffer.push_str("INNER ");
                }
            }
            JoinType::JOIN_LEFT => buffer.push_str("LEFT "),
            JoinType::JOIN_FULL => buffer.push_str("FULL "),
            JoinType::JOIN_RIGHT => buffer.push_str("RIGHT "),

            // Unused by parser
            JoinType::JOIN_SEMI => {}
            JoinType::JOIN_ANTI => {}
            JoinType::JOIN_UNIQUE_OUTER => {}
            JoinType::JOIN_UNIQUE_INNER => {}
        }
        buffer.push_str("JOIN ");

        // We need additional parenthesis if there is another join
        let join_parens = match **join_ref {
            Node::JoinExpr(ref inner) => inner.alias.is_none(),
            _ => false,
        };

        // Do the join
        if join_parens {
            buffer.push('(');
        }
        TableRef(&**join_ref).build(buffer)?;
        if join_parens {
            buffer.push(')');
        }

        // Add in the qualifying join columns
        if let Some(ref qualifiers) = self.quals {
            buffer.push_str(" ON ");
            Expr(&**qualifiers).build(buffer)?;
        }

        // Do the using, if any
        if !using.is_empty() {
            buffer.push_str(" USING (");
            NameList(using).build(buffer)?;
            buffer.push(')');
        }

        if self.alias.is_some() {
            buffer.push(')');
        }

        if let Some(ref alias) = self.alias {
            (**alias).build(buffer)?;
        }

        Ok(())
    }
}

impl SqlBuilder for MinMaxExpr {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match *self.op {
            MinMaxOp::IS_GREATEST => buffer.push_str("GREATEST("),
            MinMaxOp::IS_LEAST => buffer.push_str("LEAST("),
        }
        if let Some(ref args) = self.args {
            ExprList(args).build(buffer)?;
        }
        buffer.push(')');
        Ok(())
    }
}

impl SqlBuilder for NullTest {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let arg = must!(self.arg);
        Expr(&**arg).build(buffer)?;
        match *self.nulltesttype {
            NullTestType::IS_NULL => buffer.push_str(" IS NULL"),
            NullTestType::IS_NOT_NULL => buffer.push_str(" IS NOT NULL"),
        }
        Ok(())
    }
}

impl SqlBuilderWithContext for RangeVar {
    fn build_with_context(&self, buffer: &mut String, context: Context) -> Result<(), SqlError> {
        if !self.inh {
            match context {
                Context::CreateType | Context::AlterType => {}
                _ => buffer.push_str("ONLY "),
            }
        }

        if let Some(ref catalog) = self.catalogname {
            buffer.push_str(&format!("{}.", quote_identifier(catalog)));
        }
        if let Some(ref schema) = self.schemaname {
            buffer.push_str(&format!("{}.", quote_identifier(schema)));
        }
        if let Some(ref name) = self.relname {
            buffer.push_str(&quote_identifier(name));
        }

        // Now parse the alias
        if let Some(ref alias) = self.alias {
            if context == Context::InsertRelation {
                buffer.push_str(" AS ");
            } else {
                buffer.push(' ');
            }
            (**alias).build(buffer)?;
        }
        Ok(())
    }
}

impl SqlBuilder for RowExpr {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let args = must!(self.args);
        match *self.row_format {
            CoercionForm::COERCE_EXPLICIT_CALL => buffer.push_str("ROW"),
            CoercionForm::COERCE_EXPLICIT_CAST => {
                return Err(SqlError::Unsupported("COERCE_EXPLICIT_CAST".into()))
            }
            CoercionForm::COERCE_IMPLICIT_CAST => {}
        }
        buffer.push('(');
        ExprList(args).build(buffer)?;
        buffer.push(')');
        Ok(())
    }
}

impl SqlBuilder for SQLValueFunction {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match *self.op {
            SQLValueFunctionOp::SVFOP_CURRENT_DATE => buffer.push_str("current_date"),
            SQLValueFunctionOp::SVFOP_CURRENT_TIME => buffer.push_str("current_time"),
            SQLValueFunctionOp::SVFOP_CURRENT_TIME_N => buffer.push_str("current_time"),
            SQLValueFunctionOp::SVFOP_CURRENT_TIMESTAMP => buffer.push_str("current_timestamp"),
            SQLValueFunctionOp::SVFOP_CURRENT_TIMESTAMP_N => buffer.push_str("current_timestamp"),
            SQLValueFunctionOp::SVFOP_LOCALTIME => buffer.push_str("localtime"),
            SQLValueFunctionOp::SVFOP_LOCALTIME_N => buffer.push_str("localtime"),
            SQLValueFunctionOp::SVFOP_LOCALTIMESTAMP => buffer.push_str("localtimestamp"),
            SQLValueFunctionOp::SVFOP_LOCALTIMESTAMP_N => buffer.push_str("localtimestamp"),
            SQLValueFunctionOp::SVFOP_CURRENT_ROLE => buffer.push_str("current_role"),
            SQLValueFunctionOp::SVFOP_CURRENT_USER => buffer.push_str("current_user"),
            SQLValueFunctionOp::SVFOP_USER => buffer.push_str("user"),
            SQLValueFunctionOp::SVFOP_SESSION_USER => buffer.push_str("session_user"),
            SQLValueFunctionOp::SVFOP_CURRENT_CATALOG => buffer.push_str("current_catalog"),
            SQLValueFunctionOp::SVFOP_CURRENT_SCHEMA => buffer.push_str("current_schema"),
        }

        // Add in modifiers as necessary
        if self.typmod != -1 {
            buffer.push_str(&format!("({})", self.typmod));
        }
        Ok(())
    }
}

impl SqlBuilder for SetToDefault {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        buffer.push_str("DEFAULT");
        Ok(())
    }
}

impl SqlBuilder for SubLink {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        let sub_select = must!(self.subselect);
        match *self.sub_link_type {
            SubLinkType::EXISTS_SUBLINK => {
                buffer.push_str("EXISTS (");
                sub_select.build(buffer)?;
                buffer.push(')');
            }
            SubLinkType::ALL_SUBLINK => {
                let test = must!(self.testexpr);
                let op = must!(self.oper_name);

                // Build ALL sublink query
                Expr(&**test).build(buffer)?;
                buffer.push(' ');
                SubqueryOperator(op).build(buffer)?;
                buffer.push_str(" ALL (");
                sub_select.build(buffer)?;
                buffer.push(')');
            }
            SubLinkType::ANY_SUBLINK => {
                let test = must!(self.testexpr);

                // Start with the test
                Expr(&**test).build(buffer)?;

                // Do the operation
                if let Some(ref op) = self.oper_name {
                    buffer.push(' ');
                    SubqueryOperator(op).build(buffer)?;
                    buffer.push_str(" ANY ");
                } else {
                    buffer.push_str(" IN ");
                }
                buffer.push('(');
                sub_select.build(buffer)?;
                buffer.push(')');
            }
            SubLinkType::ROWCOMPARE_SUBLINK => {
                return Err(SqlError::Unsupported("ROWCOMPARE_SUBLINK".into()))
            }
            SubLinkType::EXPR_SUBLINK => {
                buffer.push('(');
                sub_select.build(buffer)?;
                buffer.push(')');
            }
            SubLinkType::MULTIEXPR_SUBLINK => {
                return Err(SqlError::Unsupported("MULTIEXPR_SUBLINK".into()))
            }
            SubLinkType::ARRAY_SUBLINK => {
                buffer.push_str("ARRAY(");
                sub_select.build(buffer)?;
                buffer.push(')');
            }
            SubLinkType::CTE_SUBLINK => {
                // Sub plans only
                return Err(SqlError::Unsupported("CTE_SUBLINK".into()));
            }
        }
        Ok(())
    }
}

impl SqlBuilder for XmlExpr {
    fn build(&self, buffer: &mut String) -> core::result::Result<(), SqlError> {
        match *self.op {
            XmlExprOp::IS_XMLCONCAT => {
                buffer.push_str("xmlconcat(");
                if let Some(ref args) = self.args {
                    ExprList(args).build(buffer)?;
                }
                buffer.push(')');
            }
            XmlExprOp::IS_XMLELEMENT => {
                buffer.push_str("xmlelement(name ");
                if let Some(ref name) = self.name {
                    buffer.push_str(&quote_identifier(name));
                }
                if let Some(ref named_args) = self.named_args {
                    buffer.push_str(", xmlattributes(");
                    XmlAttributeList(named_args).build(buffer)?;
                    buffer.push(')');
                }
                if let Some(ref args) = self.args {
                    buffer.push_str(", ");
                    ExprList(args).build(buffer)?;
                }
                buffer.push(')');
            }
            XmlExprOp::IS_XMLFOREST => {
                buffer.push_str("xmlforest(");
                if let Some(ref named_args) = self.named_args {
                    XmlAttributeList(named_args).build(buffer)?;
                }
                buffer.push(')');
            }
            XmlExprOp::IS_XMLPARSE => {
                let args = must!(self.args);
                if args.len() != 2 {
                    return Err(SqlError::Unsupported("args.len() != 2".into()));
                }
                buffer.push_str("xmlparse(");
                match *self.xmloption {
                    XmlOptionType::XMLOPTION_DOCUMENT => buffer.push_str("document "),
                    XmlOptionType::XMLOPTION_CONTENT => buffer.push_str("content "),
                }
                Expr(&args[0]).build(buffer)?;
                if let Node::TypeCast(ref tc) = args[1] {
                    if let Some(ref inner) = tc.arg {
                        if let Node::A_Const(ref ac) = **inner {
                            if let Node::String {
                                value: Some(ref value),
                            } = ac.val.0
                            {
                                if value.eq("t") {
                                    buffer.push_str(" PRESERVE WHITESPACE");
                                }
                            }
                        }
                    }
                }
                buffer.push(')');
            }
            XmlExprOp::IS_XMLPI => {
                buffer.push_str("xmlpi(name ");
                if let Some(ref name) = self.name {
                    buffer.push_str(&quote_identifier(name));
                }
                if let Some(ref args) = self.args {
                    if !args.is_empty() {
                        // Just one arg to push?
                        buffer.push_str(", ");
                        Expr(&args[0]).build(buffer)?;
                    }
                }
                buffer.push(')');
            }
            XmlExprOp::IS_XMLROOT => {
                let mut args = must!(self.args).iter();
                let arg = args
                    .next()
                    .ok_or_else(|| SqlError::Missing("Missing element (1)".into()))?;
                buffer.push_str("xmlroot(");
                Expr(arg).build(buffer)?;
                buffer.push_str(", version ");
                let arg = args
                    .next()
                    .ok_or_else(|| SqlError::Missing("Missing element (2)".into()))?;
                match arg {
                    Node::A_Const(a_const) => {
                        if let Node::Null {} = (*a_const.val).0 {
                            buffer.push_str("NO VALUE");
                        } else {
                            Expr(arg).build(buffer)?;
                        }
                    }
                    other => Expr(other).build(buffer)?,
                }

                let arg = args
                    .next()
                    .ok_or_else(|| SqlError::Missing("Missing element (3)".into()))?;
                let arg = node!(arg, Node::A_Const);
                let value = int_value!((*arg.val).0);

                // Guessing a bit here
                if value == 0 {
                    // XML_STANDALONE_YES
                    buffer.push_str(", STANDALONE YES");
                } else if value == 1 {
                    // XML_STANDALONE_NO
                    buffer.push_str(", STANDALONE NO");
                } else if value == 2 {
                    // XML_STANDALONE_NO_VALUE
                    buffer.push_str(", STANDALONE NO VALUE");
                }
                buffer.push(')');
            }
            XmlExprOp::IS_XMLSERIALIZE => {
                // These are represented as XmlSerialize in raw parse trees
            }
            XmlExprOp::IS_DOCUMENT => {
                let args = must!(self.args);
                if args.len() != 1 {
                    return Err(SqlError::Unsupported("args.len() != 1".into()));
                }
                Expr(&args[0]).build(buffer)?;
                buffer.push_str(" IS DOCUMENT");
            }
        }
        Ok(())
    }
}
