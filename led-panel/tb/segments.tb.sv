`timescale 1ns / 1ps

import test_cases::*;

module segments_tb;
    test_case current_test;

    digit ls_10, ls_1, rs_10, rs_1, m_10, m_1, s_10, s_1;
    logic white_on_left, white_on_right;
    logic left_to_ind, right_to_ind, ref_to_ind;
    logic one, slash, two, overtime, sdn_dth;
    logic colon;
    logic [1:0] brightness;

    logic test_failed = 0;

    segments segments(
        .data(current_test.data),
        .ls_10(ls_10),
        .ls_1(ls_1),
        .rs_10(rs_10),
        .rs_1(rs_1),
        .m_10(m_10),
        .m_1(m_1),
        .s_10(s_10),
        .s_1(s_1),
        .white_on_left(white_on_left),
        .white_on_right(white_on_right),
        .left_to_ind(left_to_ind),
        .right_to_ind(right_to_ind),
        .ref_to_ind(ref_to_ind),
        .one(one),
        .slash(slash),
        .two(two),
        .overtime(overtime),
        .sdn_dth(sdn_dth),
        .colon(colon),
        .brightness(brightness)
    );

    initial begin
        foreach (all_tests[i]) begin
            current_test = all_tests[i];
            test_failed = 0;

            #10; // Wait for 10 time units

            if (ls_10 !== current_test.ls_10_ex) begin
                $error("Test \"%s\" failed: ls_10 is %b, expected %b", current_test.name, ls_10, current_test.ls_10_ex);
                test_failed = 1;
            end
            if (ls_1 !== current_test.ls_1_ex) begin
                $error("Test \"%s\" failed: ls_1 is %b, expected %b", current_test.name, ls_1, current_test.ls_1_ex);
                test_failed = 1;
            end
            if (rs_10 !== current_test.rs_10_ex) begin
                $error("Test \"%s\" failed: rs_10 is %b, expected %b", current_test.name, rs_10, current_test.rs_10_ex);
                test_failed = 1;
            end
            if (rs_1 !== current_test.rs_1_ex) begin
                $error("Test \"%s\" failed: rs_1 is %b, expected %b", current_test.name, rs_1, current_test.rs_1_ex);
                test_failed = 1;
            end
            if (m_10 !== current_test.m_10_ex) begin
                $error("Test \"%s\" failed: m_10 is %b, expected %b", current_test.name, m_10, current_test.m_10_ex);
                test_failed = 1;
            end
            if (m_1 !== current_test.m_1_ex) begin
                $error("Test \"%s\" failed: m_1 is %b, expected %b", current_test.name, m_1, current_test.m_1_ex);
                test_failed = 1;
            end
            if (s_10 !== current_test.s_10_ex) begin
                $error("Test \"%s\" failed: s_10 is %b, expected %b", current_test.name, s_10, current_test.s_10_ex);
                test_failed = 1;
            end
            if (s_1 !== current_test.s_1_ex) begin
                $error("Test \"%s\" failed: s_1 is %b, expected %b", current_test.name, s_1, current_test.s_1_ex);
                test_failed = 1;
            end
            if (white_on_left !== current_test.white_on_left_ex) begin
                $error("Test \"%s\" failed: white_on_left is %b, expected %b", current_test.name, white_on_left, current_test.white_on_left_ex);
                test_failed = 1;
            end
            if (white_on_right !== current_test.white_on_right_ex) begin
                $error("Test \"%s\" failed: white_on_right is %b, expected %b", current_test.name, white_on_right, current_test.white_on_right_ex);
                test_failed = 1;
            end
            if (left_to_ind !== current_test.left_to_ind_ex) begin
                $error("Test \"%s\" failed: left_to_ind is %b, expected %b", current_test.name, left_to_ind, current_test.left_to_ind_ex);
                test_failed = 1;
            end
            if (right_to_ind !== current_test.right_to_ind_ex) begin
                $error("Test \"%s\" failed: right_to_ind is %b, expected %b", current_test.name, right_to_ind, current_test.right_to_ind_ex);
                test_failed = 1;
            end
            if (ref_to_ind !== current_test.ref_to_ind_ex) begin
                $error("Test \"%s\" failed: ref_to_ind is %b, expected %b", current_test.name, ref_to_ind, current_test.ref_to_ind_ex);
                test_failed = 1;
            end
            if (one !== current_test.one_ex) begin
                $error("Test \"%s\" failed: one is %b, expected %b", current_test.name, one, current_test.one_ex);
                test_failed = 1;
            end
            if (slash !== current_test.slash_ex) begin
                $error("Test \"%s\" failed: slash is %b, expected %b", current_test.name, slash, current_test.slash_ex);
                test_failed = 1;
            end
            if (two !== current_test.two_ex) begin
                $error("Test \"%s\" failed: two is %b, expected %b", current_test.name, two, current_test.two_ex);
                test_failed = 1;
            end
            if (overtime !== current_test.overtime_ex) begin
                $error("Test \"%s\" failed: overtime is %b, expected %b", current_test.name, overtime, current_test.overtime_ex);
                test_failed = 1;
            end
            if (sdn_dth !== current_test.sdn_dth_ex) begin
                $error("Test \"%s\" failed: sdn_dth is %b, expected %b", current_test.name, sdn_dth, current_test.sdn_dth_ex);
                test_failed = 1;
            end
            if (colon !== current_test.colon_ex) begin
                $error("Test \"%s\" failed: colon is %b, expected %b", current_test.name, colon, current_test.colon_ex);
                test_failed = 1;
            end
            if (brightness !== current_test.brightness_ex) begin
                $error("Test \"%s\" failed: brightness is %b, expected %b", current_test.name, brightness, current_test.brightness_ex);
                test_failed = 1;
            end

            if (!test_failed) begin
                $display("Test \"%s\" Succeeded", current_test.name);
            end
        end
    end

endmodule